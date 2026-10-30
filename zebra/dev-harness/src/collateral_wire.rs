//! Restricted V4 transaction fixtures for the offline collateral profile.
//!
//! Signatures use Zebra's ZIP-243 implementation with the Sapling branch ID. This
//! historical context is fixed by the profile; it is not current Regtest consensus.
//! Vault scripts require the profile's outpoint guard and are anyone-can-spend to
//! an unmodified node. These helpers do not replace a Zcash script interpreter.

use crate::{json_error, native_error, Error};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use secp256k1::{ecdsa, Message, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Cursor, sync::Arc};
use zebra_chain::{
    block::{self, genesis::regtest_genesis_block},
    parameters::NetworkUpgrade,
    serialization::{ZcashDeserialize, ZcashSerialize},
    transaction::{HashType, Transaction},
    transparent::{Input, OutPoint, Output, Script},
};
use zsv::{Pubkey, SignedTx, VaultId, Wallet};

pub const MAX_INPUTS: usize = 64;
pub const MAX_OUTPUTS: usize = 64;
pub const MAX_TRANSACTION_BYTES: usize = 65_536;
const VAULT_MAGIC: &[u8; 8] = b"ZSVLOCK1";
const COMMITMENT_MAGIC: &[u8; 8] = b"ZSVCOL01";

/// Separate model and transparent keys for deterministic simulated-fund fixtures.
pub struct NativeWallet {
    model: SigningKey,
    native: SecretKey,
}

impl NativeWallet {
    pub fn from_seeds(model_seed: [u8; 32], native_seed: [u8; 32]) -> Result<Self, Error> {
        Ok(Self {
            model: SigningKey::from_bytes(&model_seed),
            native: SecretKey::from_secret_bytes(native_seed).map_err(native_error)?,
        })
    }

    pub fn model_wallet(&self) -> Wallet {
        Wallet::from_seed(self.model.to_bytes())
    }

    pub fn model_key(&self) -> Pubkey {
        Pubkey(self.model.verifying_key().to_bytes())
    }

    pub fn native_key(&self) -> [u8; 33] {
        PublicKey::from_secret_key(&self.native).serialize()
    }

    pub fn sign_intent(
        &self,
        domain: [u8; 32],
        parent_checkpoint: [u8; 32],
        native: &Transaction,
        model: &SignedTx,
    ) -> Result<NativeIntent, Error> {
        if model.signer != self.model_key() {
            return Err(Error::Invalid("intent signer differs from model signer"));
        }
        let bytes = intent_bytes(domain, parent_checkpoint, native, model)?;
        Ok(NativeIntent {
            signer: self.model_key(),
            signature: self.model.sign(&bytes).to_bytes().to_vec(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeIntent {
    pub signer: Pubkey,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Free([u8; 33]),
    Vault(VaultId),
    Commitment([u8; 32]),
}

pub fn p2pk_script(key: [u8; 33]) -> Result<Script, Error> {
    PublicKey::from_byte_array_compressed(key).map_err(native_error)?;
    let mut bytes = vec![33];
    bytes.extend_from_slice(&key);
    bytes.push(0xac); // OP_CHECKSIG
    Ok(Script::new(&bytes))
}

pub fn vault_script(id: VaultId) -> Script {
    let mut bytes = vec![48];
    bytes.extend_from_slice(VAULT_MAGIC);
    bytes.extend_from_slice(&id.owner.0);
    bytes.extend_from_slice(&id.local_id.to_le_bytes());
    bytes.extend_from_slice(&[0x75, 0x51]); // OP_DROP OP_TRUE; profile guard is mandatory.
    Script::new(&bytes)
}

pub fn commitment_script(digest: [u8; 32]) -> Script {
    let mut bytes = vec![0x6a, 40];
    bytes.extend_from_slice(COMMITMENT_MAGIC);
    bytes.extend_from_slice(&digest);
    Script::new(&bytes)
}

pub fn classify_script(script: &Script) -> Result<OutputKind, Error> {
    let bytes = script.as_raw_bytes();
    if bytes.len() == 35 && bytes[0] == 33 && bytes[34] == 0xac {
        let key = bytes[1..34].try_into().expect("fixed slice length");
        PublicKey::from_byte_array_compressed(key).map_err(native_error)?;
        return Ok(OutputKind::Free(key));
    }
    if bytes.len() == 51
        && bytes[0] == 48
        && &bytes[1..9] == VAULT_MAGIC
        && bytes[49..] == [0x75, 0x51]
    {
        let owner = bytes[9..41].try_into().expect("fixed slice length");
        VerifyingKey::from_bytes(&owner).map_err(native_error)?;
        let local_id = u64::from_le_bytes(bytes[41..49].try_into().expect("fixed slice length"));
        return Ok(OutputKind::Vault(VaultId::new(Pubkey(owner), local_id)));
    }
    if bytes.len() == 42 && bytes[..2] == [0x6a, 40] && &bytes[2..10] == COMMITMENT_MAGIC {
        return Ok(OutputKind::Commitment(
            bytes[10..].try_into().expect("fixed slice length"),
        ));
    }
    Err(Error::Invalid("unsupported native output script"))
}

pub fn model_commitment(model: &SignedTx) -> Result<[u8; 32], Error> {
    Ok(Sha256::digest(serde_json::to_vec(model).map_err(json_error)?).into())
}

pub fn encode_transaction(tx: &Transaction) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    tx.zcash_serialize(&mut bytes).map_err(native_error)?;
    if bytes.len() > MAX_TRANSACTION_BYTES {
        return Err(Error::Invalid("native transaction byte limit"));
    }
    Ok(bytes)
}

pub fn decode_transaction(bytes: &[u8]) -> Result<Transaction, Error> {
    if bytes.len() > MAX_TRANSACTION_BYTES {
        return Err(Error::Invalid("native transaction byte limit"));
    }
    let mut cursor = Cursor::new(bytes);
    let tx = Transaction::zcash_deserialize(&mut cursor).map_err(native_error)?;
    if cursor.position() != bytes.len() as u64 {
        return Err(Error::Invalid("trailing native transaction bytes"));
    }
    if encode_transaction(&tx)? != bytes {
        return Err(Error::Invalid("noncanonical native transaction"));
    }
    Ok(tx)
}

fn encode_v4(inputs: &[Input], outputs: &[Output], expiry: u32) -> Result<Transaction, Error> {
    if inputs.is_empty() || inputs.len() > MAX_INPUTS || outputs.len() > MAX_OUTPUTS {
        return Err(Error::Invalid("native input or output count"));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x8000_0004_u32.to_le_bytes());
    bytes.extend_from_slice(&0x892f_2085_u32.to_le_bytes());
    // Both profile limits are below the CompactSize single-byte threshold.
    bytes.push(inputs.len() as u8);
    for input in inputs {
        input.zcash_serialize(&mut bytes).map_err(native_error)?;
    }
    bytes.push(outputs.len() as u8);
    for output in outputs {
        output.zcash_serialize(&mut bytes).map_err(native_error)?;
    }
    bytes.extend_from_slice(&0_u32.to_le_bytes()); // nLockTime
    bytes.extend_from_slice(&expiry.to_le_bytes());
    bytes.extend_from_slice(&0_i64.to_le_bytes()); // valueBalanceSapling
    bytes.extend_from_slice(&[0, 0, 0]); // Sapling spends, Sapling outputs, JoinSplits
    decode_transaction(&bytes)
}

pub fn native_transaction(
    inputs: &[OutPoint],
    outputs: &[Output],
    expiry_height: u32,
) -> Result<Transaction, Error> {
    if inputs.is_empty() || inputs.len() > MAX_INPUTS {
        return Err(Error::Invalid("native input count"));
    }
    let inputs: Vec<_> = inputs
        .iter()
        .map(|outpoint| Input::PrevOut {
            outpoint: *outpoint,
            unlock_script: Script::new(&[]),
            sequence: u32::MAX,
        })
        .collect();
    let tx = encode_v4(&inputs, outputs, expiry_height)?;
    check_user_shape(&tx)?;
    Ok(tx)
}

fn check_user_shape(tx: &Transaction) -> Result<(), Error> {
    if tx.version() != 4 || tx.has_shielded_data() || tx.raw_lock_time() != 0 {
        return Err(Error::Invalid("transparent V4 profile required"));
    }
    if !tx
        .expiry_height()
        .is_some_and(|height| (1..500_000_000).contains(&height.0))
    {
        return Err(Error::Invalid(
            "positive bounded native expiry height required",
        ));
    }
    let inputs = tx.inputs();
    let outputs = tx.outputs();
    if inputs.is_empty()
        || inputs.len() > MAX_INPUTS
        || outputs.is_empty()
        || outputs.len() > MAX_OUTPUTS
    {
        return Err(Error::Invalid("native input or output count"));
    }
    for input in inputs {
        if !matches!(
            input,
            Input::PrevOut {
                sequence: u32::MAX,
                ..
            }
        ) {
            return Err(Error::Invalid("ordinary final-sequence inputs required"));
        }
    }
    let mut commitments = 0;
    for output in outputs {
        if matches!(
            classify_script(&output.lock_script)?,
            OutputKind::Commitment(_)
        ) {
            if u64::from(output.value) != 0 {
                return Err(Error::Invalid("nonzero native commitment"));
            }
            commitments += 1;
        }
    }
    if commitments != 1 {
        return Err(Error::Invalid("one native model commitment required"));
    }
    Ok(())
}

fn resolved_inputs(tx: &Transaction, prevouts: &[Output]) -> Result<Vec<Input>, Error> {
    check_user_shape(tx)?;
    let inputs = tx.inputs();
    if inputs.len() != prevouts.len() {
        return Err(Error::Invalid("resolved prevout count"));
    }
    Ok(inputs)
}

fn signature_message(
    tx: &Transaction,
    prevouts: Arc<Vec<Output>>,
    index: usize,
) -> Result<Message, Error> {
    let script = prevouts[index].lock_script.as_raw_bytes().to_vec();
    let digest = tx
        .sighash(
            NetworkUpgrade::Sapling,
            HashType::ALL,
            prevouts,
            Some((index, script)),
        )
        .map_err(native_error)?;
    Ok(Message::from_digest(digest.0))
}

/// Sign P2PK inputs; vault inputs remain empty for the stateful profile guard.
pub fn sign_p2pk_inputs(
    wallet: &NativeWallet,
    tx: &Transaction,
    prevouts: &[Output],
) -> Result<Transaction, Error> {
    let mut inputs = resolved_inputs(tx, prevouts)?;
    let resolved = Arc::new(prevouts.to_vec());
    for (index, input) in inputs.iter_mut().enumerate() {
        let Input::PrevOut { unlock_script, .. } = input else {
            return Err(Error::Invalid("coinbase user input"));
        };
        match classify_script(&prevouts[index].lock_script)? {
            OutputKind::Free(key) => {
                if key != wallet.native_key() {
                    return Err(Error::Invalid("native signing key differs from prevout"));
                }
                let signature = ecdsa::sign(
                    signature_message(tx, resolved.clone(), index)?,
                    &wallet.native,
                );
                let der = signature.serialize_der();
                let mut bytes = vec![(der.len() + 1) as u8];
                bytes.extend_from_slice(&der);
                bytes.push(1); // Exact SIGHASH_ALL.
                *unlock_script = Script::new(&bytes);
            }
            OutputKind::Vault(_) => *unlock_script = Script::new(&[]),
            OutputKind::Commitment(_) => return Err(Error::Invalid("commitment output spent")),
        }
    }
    encode_v4(
        &inputs,
        &tx.outputs(),
        tx.expiry_height().expect("V4 expiry").0,
    )
}

/// Check this profile's P2PK signatures against authoritative resolved outputs.
/// Vault authorization and model/native value matching are the caller's stateful checks.
pub fn verify_transaction(tx: &Transaction, prevouts: &[Output]) -> Result<(), Error> {
    let inputs = resolved_inputs(tx, prevouts)?;
    let resolved = Arc::new(prevouts.to_vec());
    for (index, input) in inputs.iter().enumerate() {
        let Input::PrevOut { unlock_script, .. } = input else {
            return Err(Error::Invalid("coinbase user input"));
        };
        let bytes = unlock_script.as_raw_bytes();
        match classify_script(&prevouts[index].lock_script)? {
            OutputKind::Free(key) => {
                if !(10..=73).contains(&bytes.len())
                    || usize::from(bytes[0]) != bytes.len() - 1
                    || bytes.last() != Some(&1)
                {
                    return Err(Error::Invalid("minimal DER SIGHASH_ALL signature required"));
                }
                let der = &bytes[1..bytes.len() - 1];
                let signature = ecdsa::Signature::from_der(der).map_err(native_error)?;
                if signature.serialize_der().as_ref() != der {
                    return Err(Error::Invalid("noncanonical DER signature"));
                }
                let mut normalized = signature;
                normalized.normalize_s();
                if normalized != signature {
                    return Err(Error::Invalid("high-S native signature"));
                }
                let public = PublicKey::from_byte_array_compressed(key).map_err(native_error)?;
                ecdsa::verify(
                    &signature,
                    signature_message(tx, resolved.clone(), index)?,
                    &public,
                )
                .map_err(native_error)?;
            }
            OutputKind::Vault(_) if bytes.is_empty() => {}
            OutputKind::Vault(_) => return Err(Error::Invalid("vault input script must be empty")),
            OutputKind::Commitment(_) => return Err(Error::Invalid("commitment output spent")),
        }
    }
    Ok(())
}

fn intent_bytes(
    domain: [u8; 32],
    parent_checkpoint: [u8; 32],
    native: &Transaction,
    model: &SignedTx,
) -> Result<Vec<u8>, Error> {
    check_user_shape(native)?;
    let mut inputs = native.inputs();
    for input in &mut inputs {
        let Input::PrevOut { unlock_script, .. } = input else {
            return Err(Error::Invalid("coinbase intent input"));
        };
        *unlock_script = Script::new(&[]);
    }
    let unsigned = encode_v4(
        &inputs,
        &native.outputs(),
        native.expiry_height().expect("V4 expiry").0,
    )?;
    let native_bytes = encode_transaction(&unsigned)?;
    let model_bytes = serde_json::to_vec(model).map_err(json_error)?;
    if model_bytes.len() > crate::MAX_PAYLOAD_BYTES {
        return Err(Error::Invalid("model transaction byte limit"));
    }
    let mut bytes = b"zsv/native-collateral/intent/v1\0".to_vec();
    bytes.extend_from_slice(&domain);
    bytes.extend_from_slice(&parent_checkpoint);
    bytes.extend_from_slice(&(native_bytes.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&native_bytes);
    bytes.extend_from_slice(&(model_bytes.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&model_bytes);
    Ok(bytes)
}

pub fn verify_intent(
    domain: [u8; 32],
    parent_checkpoint: [u8; 32],
    native: &Transaction,
    model: &SignedTx,
    intent: &NativeIntent,
) -> Result<(), Error> {
    if intent.signer != model.signer {
        return Err(Error::Invalid("intent signer differs from model signer"));
    }
    let key = VerifyingKey::from_bytes(&intent.signer.0).map_err(native_error)?;
    let signature =
        ed25519_dalek::Signature::from_slice(&intent.signature).map_err(native_error)?;
    key.verify_strict(
        &intent_bytes(domain, parent_checkpoint, native, model)?,
        &signature,
    )
    .map_err(native_error)
}

fn fixture_coinbase_outputs(height: u32, outputs: &[Output]) -> Result<Transaction, Error> {
    if height == 0 {
        return Err(Error::Invalid("positive fixture coinbase height required"));
    }
    encode_v4(
        &[Input::Coinbase {
            height: block::Height(height),
            data: b"ZSV fixture".to_vec(),
            sequence: u32::MAX,
        }],
        outputs,
        0,
    )
}

/// Trusted initial allocation fixture; no subsidy or maturity validation is claimed.
pub fn fixture_funding(outputs: Vec<Output>) -> Result<Transaction, Error> {
    if outputs.is_empty() {
        return Err(Error::Invalid("empty funding fixture"));
    }
    for output in &outputs {
        if !matches!(classify_script(&output.lock_script)?, OutputKind::Free(_)) {
            return Err(Error::Invalid("funding fixture requires P2PK outputs"));
        }
    }
    fixture_coinbase_outputs(1, &outputs)
}

pub fn fixture_coinbase(height: u32, digest: [u8; 32]) -> Result<Transaction, Error> {
    fixture_coinbase_outputs(
        height,
        &[Output::new(
            0_u64.try_into().map_err(native_error)?,
            commitment_script(digest),
        )],
    )
}

/// Assemble an offline block fixture. Its header does not have valid mined proof of work.
pub fn native_block(
    parent: block::Hash,
    transactions: Vec<Arc<Transaction>>,
    nonce: u64,
) -> block::Block {
    let mut header = *regtest_genesis_block().header;
    header.previous_block_hash = parent;
    header.merkle_root = transactions.iter().collect();
    let mut nonce_bytes = [0; 32];
    nonce_bytes[..8].copy_from_slice(&nonce.to_le_bytes());
    header.nonce = nonce_bytes.into();
    block::Block {
        header: Arc::new(header),
        transactions,
    }
}
