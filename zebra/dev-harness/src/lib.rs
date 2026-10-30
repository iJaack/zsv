//! Offline experiment using Zebra's native decoder, block hashes and transaction Merkle root.
//!
//! A zero-value coinbase output commits signed model data. This does not lock native ZEC,
//! validate PoW/scripts/subsidies, create a native asset, or modify any Zebra node service.
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt, io::Cursor, sync::Arc};
use zebra_chain::{
    block::{self, genesis::regtest_genesis_block, merkle},
    parameters::Network,
    serialization::{ZcashDeserialize, ZcashSerialize},
    transaction::Transaction,
};
use zsv::{Block as ModelBlock, State};

pub const SOURCE_PIN: &str = "5faf8a413f0d30a6e93115f3d16af1e229fd7858";
pub const MAX_PAYLOAD_BYTES: usize = 1_048_576;
pub const MAX_NATIVE_BYTES: usize = 2_000_000;
pub const MAX_ENVELOPE_BYTES: usize = MAX_PAYLOAD_BYTES + MAX_NATIVE_BYTES + 48;
pub const MAX_BRANCH_BLOCKS: usize = 256;
pub const MAX_JOURNAL_BYTES: usize = 16_777_216;
const MAGIC: &[u8; 8] = b"ZSVDEV01";
const JOURNAL_MAGIC: &[u8; 8] = b"ZSVLOG01";

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Invalid(&'static str),
    Native(String),
    Json(String),
    Model(zsv::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<zsv::Error> for Error {
    fn from(error: zsv::Error) -> Self {
        Self::Model(error)
    }
}

fn native_error(error: impl fmt::Display) -> Error {
    Error::Native(error.to_string())
}
fn json_error(error: serde_json::Error) -> Error {
    Error::Json(error.to_string())
}
fn take<'a>(input: &mut &'a [u8], count: usize) -> Result<&'a [u8], Error> {
    if input.len() < count {
        return Err(Error::Invalid("truncated frame"));
    }
    let (value, rest) = input.split_at(count);
    *input = rest;
    Ok(value)
}
fn integer(input: &mut &[u8]) -> Result<usize, Error> {
    let bytes: [u8; 4] = take(input, 4)?
        .try_into()
        .map_err(|_| Error::Invalid("length"))?;
    usize::try_from(u32::from_le_bytes(bytes)).map_err(|_| Error::Invalid("length overflow"))
}
fn length(output: &mut Vec<u8>, value: usize) -> Result<(), Error> {
    let value = u32::try_from(value).map_err(|_| Error::Invalid("length overflow"))?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}
fn hash32(input: &mut &[u8]) -> Result<[u8; 32], Error> {
    take(input, 32)?
        .try_into()
        .map_err(|_| Error::Invalid("hash length"))
}
fn commitment(domain: [u8; 32], parent: [u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"zsv/zebra-dev/commitment/v1\0");
    hash.update(domain);
    hash.update(parent);
    // The byte limit is much smaller than u64::MAX on every supported host.
    hash.update((payload.len() as u64).to_le_bytes());
    hash.update(payload);
    hash.finalize().into()
}
fn commitment_script(digest: [u8; 32]) -> Vec<u8> {
    let mut script = vec![0x6a, 40]; // OP_RETURN followed by one canonical 40-byte push.
    script.extend_from_slice(MAGIC);
    script.extend_from_slice(&digest);
    script
}

struct Envelope {
    native: block::Block,
    model: ModelBlock,
    payload: Vec<u8>,
    parent_checkpoint: [u8; 32],
}
impl Envelope {
    fn decode(encoded: &[u8]) -> Result<Self, Error> {
        if encoded.len() > MAX_ENVELOPE_BYTES {
            return Err(Error::Invalid("envelope byte limit"));
        }
        let mut input = encoded;
        if take(&mut input, 8)? != MAGIC {
            return Err(Error::Invalid("envelope version"));
        }
        let parent_checkpoint = hash32(&mut input)?;
        let native_length = integer(&mut input)?;
        if native_length > MAX_NATIVE_BYTES {
            return Err(Error::Invalid("native byte limit"));
        }
        let native_bytes = take(&mut input, native_length)?;
        let payload_length = integer(&mut input)?;
        if payload_length > MAX_PAYLOAD_BYTES {
            return Err(Error::Invalid("payload byte limit"));
        }
        let payload = take(&mut input, payload_length)?;
        if !input.is_empty() {
            return Err(Error::Invalid("trailing envelope bytes"));
        }
        let mut cursor = Cursor::new(native_bytes);
        let native = block::Block::zcash_deserialize(&mut cursor).map_err(native_error)?;
        if cursor.position()
            != u64::try_from(native_bytes.len()).map_err(|_| Error::Invalid("length overflow"))?
        {
            return Err(Error::Invalid("trailing native bytes"));
        }
        let mut canonical_native = Vec::new();
        native
            .zcash_serialize(&mut canonical_native)
            .map_err(native_error)?;
        if canonical_native != native_bytes {
            return Err(Error::Invalid("noncanonical native block"));
        }
        let model: ModelBlock = serde_json::from_slice(payload).map_err(json_error)?;
        if serde_json::to_vec(&model).map_err(json_error)? != payload {
            return Err(Error::Invalid("noncanonical model payload"));
        }
        Ok(Self {
            native,
            model,
            payload: payload.to_vec(),
            parent_checkpoint,
        })
    }
    fn validate(&self, parent: &State) -> Result<State, Error> {
        if self.parent_checkpoint != parent.commitment() {
            return Err(Error::Invalid("parent model checkpoint"));
        }
        // Restrict this data-commitment experiment to one V1 coinbase. Its txid binds
        // all transaction bytes, unlike V5 txids that exclude authorizing data.
        if self.native.transactions.len() != 1 {
            return Err(Error::Invalid("one fixture coinbase required"));
        }
        let coinbase = &self.native.transactions[0];
        if coinbase.version() != 1 || !coinbase.is_coinbase() || coinbase.inputs().len() != 1 {
            return Err(Error::Invalid("V1 fixture coinbase required"));
        }
        let height = self
            .native
            .coinbase_height()
            .ok_or(Error::Invalid("coinbase height missing"))?;
        if u64::from(height.0) != self.model.height {
            return Err(Error::Invalid("native and model heights differ"));
        }
        let root: merkle::Root = self.native.transactions.iter().collect();
        if self.native.header.merkle_root != root {
            return Err(Error::Invalid("transaction merkle root"));
        }
        let outputs = coinbase.outputs();
        let expected = commitment_script(commitment(
            parent.chain_domain(),
            self.parent_checkpoint,
            &self.payload,
        ));
        if outputs.len() != 1
            || u64::from(outputs[0].value) != 0
            || outputs[0].lock_script.as_raw_bytes() != expected
        {
            return Err(Error::Invalid(
                "unique zero-value coinbase commitment required",
            ));
        }
        parent.validate_block(&self.model).map_err(Error::Model)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Node {
    state: State,
    envelope: Vec<u8>,
}
/// Keep this receipt separately from untrusted journal bytes when checking archive recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalAnchor {
    pub tip: block::Hash,
    pub sha256: [u8; 32],
}
/// Bounded in-memory branch archive. Branch selection is explicit, without a chain-work rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harness {
    nodes: BTreeMap<[u8; 32], Node>,
    order: Vec<[u8; 32]>,
    tip: block::Hash,
    genesis_checkpoint: [u8; 32],
    chain_domain: [u8; 32],
    journal_bytes: usize,
}
impl Harness {
    pub fn new(network: &Network, genesis: State) -> Result<Self, Error> {
        if !network.is_regtest() || *network != Network::new_regtest(Default::default()) {
            return Err(Error::Invalid(
                "default isolated regtest parameters required",
            ));
        }
        if genesis.snapshot().height != 0 {
            return Err(Error::Invalid("model genesis height must be zero"));
        }
        genesis.check_invariants()?;
        let tip = regtest_genesis_block().hash();
        let genesis_checkpoint = genesis.commitment();
        let chain_domain = genesis.chain_domain();
        Ok(Self {
            nodes: BTreeMap::from([(
                tip.0,
                Node {
                    state: genesis,
                    envelope: vec![],
                },
            )]),
            order: vec![],
            tip,
            genesis_checkpoint,
            chain_domain,
            journal_bytes: 108,
        })
    }
    pub fn tip(&self) -> block::Hash {
        self.tip
    }
    pub fn state_at(&self, hash: block::Hash) -> Option<State> {
        self.nodes.get(&hash.0).map(|node| node.state.clone())
    }
    pub fn envelope_at(&self, hash: block::Hash) -> Option<&[u8]> {
        self.nodes.get(&hash.0).map(|node| node.envelope.as_slice())
    }
    pub fn select_tip(&mut self, hash: block::Hash) -> Result<(), Error> {
        if !self.nodes.contains_key(&hash.0) {
            return Err(Error::Invalid("unknown branch tip"));
        }
        self.tip = hash;
        Ok(())
    }
    /// Check the native commitment and execute signatures against the exact parent branch.
    /// Any failure leaves the complete archive and selected tip unchanged.
    pub fn accept(&mut self, encoded: &[u8]) -> Result<block::Hash, Error> {
        if self.order.len() >= MAX_BRANCH_BLOCKS {
            return Err(Error::Invalid("branch archive limit"));
        }
        let journal_bytes = self
            .journal_bytes
            .checked_add(4)
            .and_then(|n| n.checked_add(encoded.len()))
            .ok_or(Error::Invalid("journal length overflow"))?;
        if journal_bytes > MAX_JOURNAL_BYTES {
            return Err(Error::Invalid("journal byte limit"));
        }
        let envelope = Envelope::decode(encoded)?;
        let hash = envelope.native.hash();
        if self.nodes.contains_key(&hash.0) {
            return Err(Error::Invalid("duplicate native block"));
        }
        let parent_hash = envelope.native.header.previous_block_hash;
        let parent = self
            .nodes
            .get(&parent_hash.0)
            .ok_or(Error::Invalid("unknown native parent"))?;
        let state = envelope.validate(&parent.state)?;
        self.nodes.insert(
            hash.0,
            Node {
                state,
                envelope: encoded.to_vec(),
            },
        );
        self.order.push(hash.0);
        self.journal_bytes = journal_bytes;
        self.tip = hash;
        Ok(hash)
    }
    /// Export accepted envelopes, never mutable snapshots. This is not a crash-safe database.
    pub fn journal(&self) -> Result<Vec<u8>, Error> {
        let bytes = self.order.iter().try_fold(108usize, |total, hash| {
            total
                .checked_add(4 + self.nodes[hash].envelope.len())
                .ok_or(Error::Invalid("journal length overflow"))
        })?;
        if bytes > MAX_JOURNAL_BYTES {
            return Err(Error::Invalid("journal byte limit"));
        }
        let mut output = Vec::with_capacity(bytes);
        output.extend_from_slice(JOURNAL_MAGIC);
        output.extend_from_slice(&self.chain_domain);
        output.extend_from_slice(&self.genesis_checkpoint);
        output.extend_from_slice(&self.tip.0);
        length(&mut output, self.order.len())?;
        for hash in &self.order {
            let encoded = &self.nodes[hash].envelope;
            length(&mut output, encoded.len())?;
            output.extend_from_slice(encoded);
        }
        Ok(output)
    }
    pub fn journal_anchor(&self) -> Result<JournalAnchor, Error> {
        Ok(JournalAnchor {
            tip: self.tip,
            sha256: Sha256::digest(self.journal()?).into(),
        })
    }
    /// Revalidate every envelope from a trusted fixture genesis and external archive receipt.
    /// The receipt authenticates both the selected branch and every retained sibling.
    pub fn replay(
        network: &Network,
        genesis: State,
        encoded: &[u8],
        anchor: JournalAnchor,
    ) -> Result<Self, Error> {
        if encoded.len() > MAX_JOURNAL_BYTES {
            return Err(Error::Invalid("journal byte limit"));
        }
        let digest: [u8; 32] = Sha256::digest(encoded).into();
        if digest != anchor.sha256 {
            return Err(Error::Invalid("journal digest mismatch"));
        }
        let mut input = encoded;
        if take(&mut input, 8)? != JOURNAL_MAGIC {
            return Err(Error::Invalid("journal version"));
        }
        let domain = hash32(&mut input)?;
        let genesis_checkpoint = hash32(&mut input)?;
        let tip = block::Hash(hash32(&mut input)?);
        let count = integer(&mut input)?;
        if count > MAX_BRANCH_BLOCKS {
            return Err(Error::Invalid("branch archive limit"));
        }
        let mut archive = Self::new(network, genesis)?;
        if domain != archive.chain_domain
            || genesis_checkpoint != archive.genesis_checkpoint
            || tip != anchor.tip
        {
            return Err(Error::Invalid("journal trust anchor mismatch"));
        }
        for _ in 0..count {
            let size = integer(&mut input)?;
            if size > MAX_ENVELOPE_BYTES {
                return Err(Error::Invalid("envelope byte limit"));
            }
            archive.accept(take(&mut input, size)?)?;
        }
        if !input.is_empty() {
            return Err(Error::Invalid("trailing journal bytes"));
        }
        archive.select_tip(tip)?;
        Ok(archive)
    }
}

/// Construct an unmined development fixture with a real serialized V1 coinbase commitment.
/// There is no native spendable balance or collateral lock in this fixture.
pub fn fixture_envelope(
    parent: &State,
    parent_hash: block::Hash,
    model: &ModelBlock,
    nonce: u64,
) -> Result<Vec<u8>, Error> {
    let payload = serde_json::to_vec(model).map_err(json_error)?;
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(Error::Invalid("payload byte limit"));
    }
    let digest = commitment(parent.chain_domain(), parent.commitment(), &payload);
    let height = u32::try_from(model.height).map_err(|_| Error::Invalid("fixture height range"))?;
    let mut height_script = if (1..=16).contains(&height) {
        vec![u8::try_from(height).map_err(|_| Error::Invalid("fixture height range"))? + 0x50]
    } else if height > 0 {
        let mut number = height.to_le_bytes().to_vec();
        while number.last() == Some(&0) {
            number.pop();
        }
        if number.last().is_some_and(|byte| byte & 0x80 != 0) {
            number.push(0);
        }
        let mut push =
            vec![u8::try_from(number.len()).map_err(|_| Error::Invalid("height script length"))?];
        push.extend(number);
        push
    } else {
        return Err(Error::Invalid("fixture height must be positive"));
    };
    height_script.push(0); // Keep the coinbase script at least two bytes long.
    let script = commitment_script(digest);
    let mut raw = 1u32.to_le_bytes().to_vec();
    raw.push(1); // One input.
    raw.extend_from_slice(&[0; 32]);
    raw.extend_from_slice(&u32::MAX.to_le_bytes());
    raw.push(
        u8::try_from(height_script.len()).map_err(|_| Error::Invalid("height script length"))?,
    );
    raw.extend(height_script);
    raw.extend_from_slice(&u32::MAX.to_le_bytes());
    raw.push(1); // One zero-value output.
    raw.extend_from_slice(&0u64.to_le_bytes());
    raw.push(u8::try_from(script.len()).map_err(|_| Error::Invalid("commitment script length"))?);
    raw.extend(script);
    raw.extend_from_slice(&0u32.to_le_bytes());
    let transaction =
        Arc::new(Transaction::zcash_deserialize(raw.as_slice()).map_err(native_error)?);
    let transactions = vec![transaction];
    let mut header = *regtest_genesis_block().header;
    header.previous_block_hash = parent_hash;
    header.merkle_root = transactions.iter().collect();
    let mut nonce_bytes = [0u8; 32];
    nonce_bytes[..8].copy_from_slice(&nonce.to_le_bytes());
    header.nonce = nonce_bytes.into();
    let native = block::Block {
        header: Arc::new(header),
        transactions,
    };
    let mut native_bytes = Vec::new();
    native
        .zcash_serialize(&mut native_bytes)
        .map_err(native_error)?;
    let mut output = MAGIC.to_vec();
    output.extend_from_slice(&parent.commitment());
    length(&mut output, native_bytes.len())?;
    output.extend(native_bytes);
    length(&mut output, payload.len())?;
    output.extend(payload);
    Ok(output)
}
