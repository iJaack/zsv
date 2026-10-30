//! Collateral-backed, transparent development profile. No node service is activated.
//!
//! Native P2PK signatures and a mandatory experimental vault guard protect fixture
//! UTXOs here. The protocol scripts are not protected by unmodified Zcash nodes.
use crate::{
    collateral_wire::{self as wire, NativeIntent, OutputKind},
    Error,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
    sync::Arc,
};
use zebra_chain::{
    amount::{Amount, NonNegative},
    block::{self, merkle},
    parameters::Network,
    serialization::{ZcashDeserialize, ZcashSerialize},
    transaction::Transaction,
    transparent::{OutPoint, Output, Utxo},
};
use zsv::{Action, Block as ModelBlock, Pubkey, Snapshot, State, VaultId};

pub const MAX_UTXOS: usize = 4096;
pub const MAX_BLOCKS: usize = 256;
pub const MAX_ENCODED_BLOCK: usize = 3_048_624;
pub const MAX_NATIVE_BYTES: usize = 2_000_000;
pub const MAX_PAYLOAD_BYTES: usize = 1_048_576;
const MAGIC: &[u8; 8] = b"ZSVCOL01";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerBinding {
    pub owner: Pubkey,
    pub native_key: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub model: ModelBlock,
    pub intents: Vec<NativeIntent>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Bucket {
    Wallet(Pubkey),
    Vault(VaultId),
}
type Point = ([u8; 32], u32);
fn point(outpoint: OutPoint) -> Point {
    (outpoint.hash.0, outpoint.index)
}
fn outpoint(point: Point) -> OutPoint {
    OutPoint {
        hash: point.0.into(),
        index: point.1,
    }
}
fn invalid(why: &'static str) -> Error {
    Error::Invalid(why)
}
fn encoded<T: ZcashSerialize>(value: &T) -> Result<Vec<u8>, Error> {
    let mut bytes = vec![];
    value
        .zcash_serialize(&mut bytes)
        .map_err(|e| Error::Native(e.to_string()))?;
    Ok(bytes)
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(value).map_err(|e| Error::Json(e.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollateralState {
    model: State,
    coins: BTreeMap<Point, Utxo>,
    owners: BTreeMap<Pubkey, [u8; 33]>,
    domain: [u8; 32],
}
impl CollateralState {
    pub fn model_state(&self) -> State {
        self.model.clone()
    }
    pub fn profile_domain(&self) -> [u8; 32] {
        self.domain
    }
    pub fn utxos(&self) -> Vec<(OutPoint, Output)> {
        self.coins
            .iter()
            .map(|(p, c)| (outpoint(*p), c.output.clone()))
            .collect()
    }
    pub fn checkpoint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"zsv/collateral-state/v1\0");
        hash.update(self.domain);
        hash.update(self.model.commitment());
        hash.update((self.coins.len() as u64).to_le_bytes());
        for (p, coin) in &self.coins {
            hash.update(p.0);
            hash.update(p.1.to_le_bytes());
            hash.update(coin.height.0.to_le_bytes());
            hash.update([u8::from(coin.from_coinbase)]);
            hash.update(encoded(&coin.output).expect("in-memory output serialization"));
        }
        hash.finalize().into()
    }
    fn bucket(&self, output: &Output) -> Result<Bucket, Error> {
        match wire::classify_script(&output.lock_script)? {
            OutputKind::Free(key) => self
                .owners
                .iter()
                .find_map(|(owner, native)| (*native == key).then_some(Bucket::Wallet(*owner)))
                .ok_or_else(|| invalid("unregistered native owner")),
            OutputKind::Vault(id) => Ok(Bucket::Vault(id)),
            OutputKind::Commitment(_) => Err(invalid("commitment is not a spendable coin")),
        }
    }
    fn reconcile(&self, model: &Snapshot) -> Result<(), Error> {
        if self.coins.len() > MAX_UTXOS {
            return Err(invalid("native UTXO limit"));
        }
        if model.reserve.is_some() {
            return Err(invalid("reserve is unsupported by collateral profile"));
        }
        if model
            .accounts
            .keys()
            .any(|owner| !self.owners.contains_key(owner))
        {
            return Err(invalid("unregistered model owner"));
        }
        let mut sums = BTreeMap::<Bucket, u64>::new();
        for coin in self.coins.values() {
            let bucket = self.bucket(&coin.output)?;
            if let Bucket::Vault(id) = &bucket {
                if !model.vaults.contains_key(id) {
                    return Err(invalid("native coin has no active vault"));
                }
            }
            let total = sums.entry(bucket).or_default();
            *total = total
                .checked_add(u64::from(coin.output.value))
                .ok_or_else(|| invalid("native bucket overflow"))?;
        }
        for (owner, account) in &model.accounts {
            if sums.remove(&Bucket::Wallet(*owner)).unwrap_or(0) != account.zec {
                return Err(invalid("wallet native collateral mismatch"));
            }
        }
        for (id, vault) in &model.vaults {
            if sums.remove(&Bucket::Vault(*id)).unwrap_or(0) != vault.collateral {
                return Err(invalid("vault native collateral mismatch"));
            }
        }
        if !sums.is_empty() {
            return Err(invalid("unaccounted native bucket"));
        }
        Ok(())
    }
    fn execute_pair(
        &mut self,
        native: &Transaction,
        model: &zsv::SignedTx,
        intent: &NativeIntent,
        post: &Snapshot,
        parent_checkpoint: [u8; 32],
    ) -> Result<(), Error> {
        let mut touched = BTreeSet::new();
        for action in &model.actions {
            match *action {
                Action::Open { vault_id, .. }
                | Action::Deposit { vault_id, .. }
                | Action::Borrow { vault_id, .. }
                | Action::Repay { vault_id, .. }
                | Action::Withdraw { vault_id, .. }
                | Action::Close { vault_id } => {
                    touched.insert(VaultId::new(model.signer, vault_id));
                }
                Action::Liquidate {
                    vault_id, owner, ..
                } => {
                    touched.insert(VaultId::new(owner, vault_id));
                }
                _ => {}
            }
        }
        let spends: Vec<_> = native.spent_outpoints().collect();
        if spends.is_empty() {
            return Err(invalid("native operation requires an input"));
        }
        let mut seen = BTreeSet::new();
        let mut previous = vec![];
        let mut input_value = 0u64;
        for spend in &spends {
            if !seen.insert(point(*spend)) {
                return Err(invalid("duplicate native spend"));
            }
            let coin = self
                .coins
                .get(&point(*spend))
                .ok_or_else(|| invalid("missing or spent native output"))?;
            match self.bucket(&coin.output)? {
                Bucket::Wallet(owner) if owner != model.signer => {
                    return Err(invalid(
                        "native wallet input owner differs from model signer",
                    ))
                }
                Bucket::Vault(id) if !touched.contains(&id) => {
                    return Err(invalid("vault input lacks an authorized action"))
                }
                _ => {}
            }
            input_value = input_value
                .checked_add(u64::from(coin.output.value))
                .ok_or_else(|| invalid("native input overflow"))?;
            previous.push(coin.output.clone());
        }
        wire::verify_transaction(native, &previous)?;
        let native_height = post
            .height
            .checked_add(1)
            .ok_or_else(|| invalid("height overflow"))?;
        if native
            .expiry_height()
            .is_none_or(|expiry| u64::from(expiry.0) < native_height)
        {
            return Err(invalid("native transaction expired"));
        }
        wire::verify_intent(self.domain, parent_checkpoint, native, model, intent)?;
        let mut outputs = vec![];
        let mut output_value = 0u64;
        let mut commitments = 0;
        let expected = wire::model_commitment(model)?;
        for (index, output) in native.outputs().into_iter().enumerate() {
            output_value = output_value
                .checked_add(u64::from(output.value))
                .ok_or_else(|| invalid("native output overflow"))?;
            match wire::classify_script(&output.lock_script)? {
                OutputKind::Commitment(digest) => {
                    if digest != expected || u64::from(output.value) != 0 {
                        return Err(invalid("native model commitment mismatch"));
                    }
                    commitments += 1;
                }
                _ => {
                    if let Bucket::Vault(id) = self.bucket(&output)? {
                        if !touched.contains(&id) {
                            return Err(invalid("vault output lacks an authorized action"));
                        }
                    }
                    outputs.push((OutPoint::from_usize(native.hash(), index), output));
                }
            }
        }
        if commitments != 1 {
            return Err(invalid("one native model commitment required"));
        }
        if input_value != output_value {
            return Err(invalid("native zero-fee value conservation"));
        }
        for spend in spends {
            self.coins.remove(&point(spend));
        }
        for (outpoint, output) in outputs {
            let height = u32::try_from(
                post.height
                    .checked_add(1)
                    .ok_or_else(|| invalid("height overflow"))?,
            )
            .map_err(|_| invalid("native height range"))?;
            if self
                .coins
                .insert(
                    point(outpoint),
                    Utxo::new(output, block::Height(height), false),
                )
                .is_some()
            {
                return Err(invalid("duplicate native output"));
            }
        }
        self.reconcile(post)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollateralHarness {
    nodes: BTreeMap<[u8; 32], CollateralState>,
    tip: block::Hash,
}
impl CollateralHarness {
    pub fn new(
        network: &Network,
        model_genesis: State,
        owners: Vec<OwnerBinding>,
        funding: Transaction,
    ) -> Result<Self, Error> {
        if *network != Network::new_regtest(Default::default()) {
            return Err(invalid("default isolated regtest parameters required"));
        }
        let snapshot = model_genesis.snapshot();
        if snapshot.height != 0 || snapshot.reserve.is_some() {
            return Err(invalid("vault-only model genesis required"));
        }
        model_genesis.check_invariants()?;
        Amount::<NonNegative>::try_from(snapshot.fixture_zec_total)
            .map_err(|_| invalid("fixture issuance exceeds native money range"))?;
        let mut bindings = BTreeMap::new();
        let mut keys = BTreeSet::new();
        for owner in owners {
            let key: [u8; 33] = owner
                .native_key
                .try_into()
                .map_err(|_| invalid("native public key length"))?;
            wire::p2pk_script(key)?;
            if bindings.insert(owner.owner, key).is_some() || !keys.insert(key) {
                return Err(invalid("owner mapping must be bijective"));
            }
        }
        if bindings.keys().ne(snapshot.accounts.keys()) {
            return Err(invalid("owner mapping differs from model genesis"));
        }
        // The funding transaction is trusted fixture issuance, but its complete bytes
        // and actual outputs are committed by this custom unmined bootstrap block.
        let expected_funding = wire::fixture_funding(funding.outputs())?;
        if encoded(&expected_funding)? != encoded(&funding)? {
            return Err(invalid("canonical fixture funding coinbase required"));
        }
        let mut coins = BTreeMap::new();
        for (index, output) in funding.outputs().into_iter().enumerate() {
            if !matches!(
                wire::classify_script(&output.lock_script)?,
                OutputKind::Free(_)
            ) {
                return Err(invalid("bootstrap outputs must be P2PK"));
            }
            coins.insert(
                point(OutPoint::from_usize(funding.hash(), index)),
                Utxo::new(output, block::Height(1), true),
            );
        }
        let root = wire::native_block(block::Hash([0; 32]), vec![Arc::new(funding)], 0);
        let mut hash = Sha256::new();
        hash.update(b"zsv/collateral-chain/v1\0");
        hash.update(model_genesis.commitment());
        hash.update((bindings.len() as u64).to_le_bytes());
        for (owner, native) in &bindings {
            hash.update(owner.0);
            hash.update(native);
        }
        hash.update(encoded(&root)?);
        let state = CollateralState {
            model: model_genesis,
            coins,
            owners: bindings,
            domain: hash.finalize().into(),
        };
        state.reconcile(&snapshot)?;
        let tip = root.hash();
        Ok(Self {
            nodes: BTreeMap::from([(tip.0, state)]),
            tip,
        })
    }
    pub fn tip(&self) -> block::Hash {
        self.tip
    }
    pub fn select_tip(&mut self, hash: block::Hash) -> Result<(), Error> {
        if !self.nodes.contains_key(&hash.0) {
            return Err(invalid("unknown collateral branch"));
        }
        self.tip = hash;
        Ok(())
    }
    pub fn state_at(&self, hash: block::Hash) -> Option<CollateralState> {
        self.nodes.get(&hash.0).cloned()
    }
    pub fn accept(&mut self, bytes: &[u8]) -> Result<block::Hash, Error> {
        if self.nodes.len() > MAX_BLOCKS {
            return Err(invalid("collateral branch limit"));
        }
        let (native, payload, parent_checkpoint) = decode(bytes)?;
        let parent = self
            .nodes
            .get(&native.header.previous_block_hash.0)
            .ok_or_else(|| invalid("unknown collateral parent"))?;
        if parent.checkpoint() != parent_checkpoint {
            return Err(invalid("collateral parent checkpoint"));
        }
        let hash = native.hash();
        if self.nodes.contains_key(&hash.0) {
            return Err(invalid("duplicate collateral block"));
        }
        let height = payload
            .model
            .height
            .checked_add(1)
            .and_then(|h| u32::try_from(h).ok())
            .ok_or_else(|| invalid("native height range"))?;
        if native.coinbase_height() != Some(block::Height(height)) {
            return Err(invalid("native collateral height"));
        }
        let expected_coinbase =
            wire::fixture_coinbase(height, block_commitment(parent, &payload)?)?;
        let first = native
            .transactions
            .first()
            .ok_or_else(|| invalid("missing commitment coinbase"))?;
        if encoded(first.as_ref())? != encoded(&expected_coinbase)? {
            return Err(invalid("collateral coinbase commitment"));
        }
        let root: merkle::Root = native.transactions.iter().collect();
        if root != native.header.merkle_root {
            return Err(invalid("collateral transaction merkle root"));
        }
        if native.transactions.len() != payload.model.transactions.len() + 1
            || payload.intents.len() != payload.model.transactions.len()
        {
            return Err(invalid(
                "every native transaction requires one signed model operation",
            ));
        }
        let mut hashes = BTreeSet::new();
        if native
            .transactions
            .iter()
            .any(|tx| !hashes.insert(tx.hash().0))
        {
            return Err(invalid("duplicate native transaction"));
        }
        let (model, traces) = parent.model.validate_block_with_trace(&payload.model)?;
        let mut candidate = parent.clone();
        candidate.reconcile(&traces[0])?;
        for (i, ((tx, action), intent)) in native.transactions[1..]
            .iter()
            .zip(&payload.model.transactions)
            .zip(&payload.intents)
            .enumerate()
        {
            candidate.execute_pair(tx, action, intent, &traces[i + 1], parent_checkpoint)?;
        }
        candidate.model = model;
        candidate.reconcile(&candidate.model.snapshot())?;
        self.nodes.insert(hash.0, candidate);
        self.tip = hash;
        Ok(hash)
    }
}
fn block_commitment(parent: &CollateralState, payload: &Payload) -> Result<[u8; 32], Error> {
    let mut hash = Sha256::new();
    hash.update(b"zsv/collateral-block/v1\0");
    hash.update(parent.profile_domain());
    hash.update(parent.checkpoint());
    hash.update(json(payload)?);
    Ok(hash.finalize().into())
}
fn take<'a>(input: &mut &'a [u8], count: usize) -> Result<&'a [u8], Error> {
    if input.len() < count {
        return Err(invalid("truncated collateral envelope"));
    }
    let (value, rest) = input.split_at(count);
    *input = rest;
    Ok(value)
}
fn size(input: &mut &[u8]) -> Result<usize, Error> {
    Ok(u32::from_le_bytes(
        take(input, 4)?
            .try_into()
            .map_err(|_| invalid("collateral length"))?,
    ) as usize)
}
fn decode(bytes: &[u8]) -> Result<(block::Block, Payload, [u8; 32]), Error> {
    if bytes.len() > MAX_ENCODED_BLOCK {
        return Err(invalid("collateral envelope byte limit"));
    }
    let mut input = bytes;
    if take(&mut input, 8)? != MAGIC {
        return Err(invalid("collateral envelope version"));
    }
    let checkpoint = take(&mut input, 32)?
        .try_into()
        .map_err(|_| invalid("collateral checkpoint length"))?;
    let native_length = size(&mut input)?;
    if native_length > MAX_NATIVE_BYTES {
        return Err(invalid("collateral native byte limit"));
    }
    let native_bytes = take(&mut input, native_length)?;
    let payload_length = size(&mut input)?;
    if payload_length > MAX_PAYLOAD_BYTES {
        return Err(invalid("collateral payload byte limit"));
    }
    let payload_bytes = take(&mut input, payload_length)?;
    if !input.is_empty() {
        return Err(invalid("trailing collateral envelope bytes"));
    }
    let mut cursor = Cursor::new(native_bytes);
    let native =
        block::Block::zcash_deserialize(&mut cursor).map_err(|e| Error::Native(e.to_string()))?;
    if cursor.position() as usize != native_bytes.len() || encoded(&native)? != native_bytes {
        return Err(invalid("noncanonical collateral native block"));
    }
    let payload: Payload =
        serde_json::from_slice(payload_bytes).map_err(|e| Error::Json(e.to_string()))?;
    if json(&payload)? != payload_bytes {
        return Err(invalid("noncanonical collateral payload"));
    }
    Ok((native, payload, checkpoint))
}
pub fn fixture_envelope(
    parent: &CollateralState,
    parent_hash: block::Hash,
    model: &ModelBlock,
    native_transactions: Vec<Transaction>,
    intents: Vec<NativeIntent>,
    nonce: u64,
) -> Result<Vec<u8>, Error> {
    let payload = Payload {
        model: model.clone(),
        intents,
    };
    let height = model
        .height
        .checked_add(1)
        .and_then(|h| u32::try_from(h).ok())
        .ok_or_else(|| invalid("native height range"))?;
    let mut transactions = vec![Arc::new(wire::fixture_coinbase(
        height,
        block_commitment(parent, &payload)?,
    )?)];
    transactions.extend(native_transactions.into_iter().map(Arc::new));
    let native = wire::native_block(parent_hash, transactions, nonce);
    let native_bytes = encoded(&native)?;
    let payload_bytes = json(&payload)?;
    if native_bytes.len() > MAX_NATIVE_BYTES || payload_bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(invalid("collateral fixture byte limit"));
    }
    let mut output = MAGIC.to_vec();
    output.extend_from_slice(&parent.checkpoint());
    output.extend_from_slice(&(native_bytes.len() as u32).to_le_bytes());
    output.extend(native_bytes);
    output.extend_from_slice(&(payload_bytes.len() as u32).to_le_bytes());
    output.extend(payload_bytes);
    Ok(output)
}
