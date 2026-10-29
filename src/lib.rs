//! Regtest-only research model. This is not a Zcash consensus implementation.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ZATOSHIS_PER_ZEC: u64 = 100_000_000;
pub const MICRO_USD: u64 = 1_000_000;
pub const MIN_DEBT: u64 = 100 * MICRO_USD;
pub const SUPPLY_CAP: u64 = 100_000 * MICRO_USD;
pub const MAX_TRANSACTIONS: usize = 256;
pub const MAX_ACTIONS_PER_TX: usize = 64;
pub const MAX_ACTIONS_PER_BLOCK: usize = 1024;
pub const MAX_ORACLE_LIFETIME: u64 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Regtest,
    Mainnet,
    Testnet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Asset {
    ZEC,
    ZUSD,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pubkey(pub [u8; 32]);
impl std::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl Serialize for Pubkey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for Pubkey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.len() != 64 {
            return Err(serde::de::Error::custom(
                "key must be 64 lowercase hex characters",
            ));
        }
        let mut bytes = [0u8; 32];
        let nibble = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        };
        for (i, pair) in text.as_bytes().chunks_exact(2).enumerate() {
            let hi = nibble(pair[0]).ok_or_else(|| serde::de::Error::custom("invalid key hex"))?;
            let lo = nibble(pair[1]).ok_or_else(|| serde::de::Error::custom("invalid key hex"))?;
            bytes[i] = hi * 16 + lo;
        }
        Ok(Self(bytes))
    }
}
/// Owner-scoped model identifier. Local IDs never reserve another owner's namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VaultId {
    pub owner: Pubkey,
    pub local_id: u64,
}
impl VaultId {
    pub fn new(owner: Pubkey, local_id: u64) -> Self {
        Self { owner, local_id }
    }
}
impl Serialize for VaultId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}:{}", self.owner, self.local_id))
    }
}
impl<'de> Deserialize<'de> for VaultId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let (owner_text, id_text) = text
            .split_once(':')
            .ok_or_else(|| serde::de::Error::custom("vault ID requires owner:local_id"))?;
        let owner = Pubkey::deserialize(serde::de::value::StrDeserializer::<D::Error>::new(
            owner_text,
        ))?;
        let local_id = id_text
            .parse::<u64>()
            .map_err(|_| serde::de::Error::custom("invalid local vault ID"))?;
        if id_text != local_id.to_string() {
            return Err(serde::de::Error::custom("noncanonical local vault ID"));
        }
        Ok(Self { owner, local_id })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub network: Network,
    pub oracle_keys: [Pubkey; 3],
}
impl Config {
    pub fn regtest(oracle_keys: [Pubkey; 3]) -> Self {
        Self {
            network: Network::Regtest,
            oracle_keys,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Action {
    Open {
        vault_id: u64,
        collateral: u64,
    },
    Deposit {
        vault_id: u64,
        amount: u64,
    },
    Borrow {
        vault_id: u64,
        amount: u64,
    },
    Repay {
        vault_id: u64,
        amount: u64,
    },
    Withdraw {
        vault_id: u64,
        amount: u64,
    },
    Close {
        vault_id: u64,
    },
    Liquidate {
        vault_id: u64,
        owner: Pubkey,
        oracle_sequence: u64,
        max_burn: u64,
        min_collateral: u64,
    },
    Transfer {
        asset: Asset,
        to: Pubkey,
        amount: u64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedTx {
    pub network: Network,
    pub chain_domain: [u8; 32],
    pub signer: Pubkey,
    pub nonce: u64,
    pub actions: Vec<Action>,
    pub signature: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedOracleReport {
    pub network: Network,
    pub chain_domain: [u8; 32],
    pub signer: Pubkey,
    pub sequence: u64,
    pub price: u64,
    pub valid_until: u64,
    pub signature: Vec<u8>,
}
pub struct OracleReport;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleUpdate {
    pub reports: Vec<SignedOracleReport>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub height: u64,
    pub oracle: Option<OracleUpdate>,
    pub transactions: Vec<SignedTx>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub zec: u64,
    pub zusd: u64,
    pub nonce: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    pub owner: Pubkey,
    pub collateral: u64,
    pub debt: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Price {
    pub sequence: u64,
    pub price: u64,
    pub valid_until: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub chain_domain: [u8; 32],
    pub height: u64,
    pub oracle: Option<Price>,
    pub accounts: BTreeMap<Pubkey, Account>,
    pub vaults: BTreeMap<VaultId, Vault>,
    pub stable_supply: u64,
    pub bad_debt: u64,
    pub fixture_zec_total: u64,
    pub used_vault_ids: BTreeSet<VaultId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    config: Config,
    inner: Snapshot,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid(&'static str),
    ArithmeticOverflow,
    InvariantViolation,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
/// Deterministic signing helper for tests/examples; not a wallet or key store.
pub struct Wallet {
    key: SigningKey,
}
impl Wallet {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }
    pub fn public_key(&self) -> Pubkey {
        Pubkey(self.key.verifying_key().to_bytes())
    }
    pub fn sign_on(&self, state: &State, nonce: u64, actions: Vec<Action>) -> SignedTx {
        self.sign_for_on(state, Network::Regtest, nonce, actions)
    }
    pub fn sign_for_on(
        &self,
        state: &State,
        network: Network,
        nonce: u64,
        actions: Vec<Action>,
    ) -> SignedTx {
        let mut tx = SignedTx {
            network,
            chain_domain: state.chain_domain(),
            signer: self.public_key(),
            nonce,
            actions,
            signature: vec![],
        };
        tx.signature = self.key.sign(&tx.signing_bytes()).to_bytes().to_vec();
        tx
    }
}
impl OracleReport {
    pub fn sign_on(
        state: &State,
        wallet: &Wallet,
        sequence: u64,
        price: u64,
        valid_until: u64,
    ) -> SignedOracleReport {
        let mut report = SignedOracleReport {
            network: Network::Regtest,
            chain_domain: state.chain_domain(),
            signer: wallet.public_key(),
            sequence,
            price,
            valid_until,
            signature: vec![],
        };
        report.signature = wallet.key.sign(&report.signing_bytes()).to_bytes().to_vec();
        report
    }
}

fn network_tag(network: Network) -> u8 {
    match network {
        Network::Regtest => 0,
        Network::Mainnet => 1,
        Network::Testnet => 2,
    }
}
fn numbers(bytes: &mut Vec<u8>, values: &[u64]) {
    for n in values {
        bytes.extend_from_slice(&n.to_le_bytes());
    }
}
impl SignedTx {
    /// Model signing encoding: version, network, immutable chain domain, key, nonce,
    /// count, then tagged actions. Liquidation includes the targeted owner's full key.
    /// Integers are unsigned 64-bit little-endian; asset tags are ZEC=0, ZUSD=1.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = b"zsv/tx/v2\0".to_vec();
        bytes.push(network_tag(self.network));
        bytes.extend_from_slice(&self.chain_domain);
        bytes.extend_from_slice(&self.signer.0);
        numbers(&mut bytes, &[self.nonce, self.actions.len() as u64]);
        for action in &self.actions {
            match action {
                Action::Open {
                    vault_id,
                    collateral,
                } => {
                    bytes.push(0);
                    numbers(&mut bytes, &[*vault_id, *collateral]);
                }
                Action::Deposit { vault_id, amount } => {
                    bytes.push(1);
                    numbers(&mut bytes, &[*vault_id, *amount]);
                }
                Action::Borrow { vault_id, amount } => {
                    bytes.push(2);
                    numbers(&mut bytes, &[*vault_id, *amount]);
                }
                Action::Repay { vault_id, amount } => {
                    bytes.push(3);
                    numbers(&mut bytes, &[*vault_id, *amount]);
                }
                Action::Withdraw { vault_id, amount } => {
                    bytes.push(4);
                    numbers(&mut bytes, &[*vault_id, *amount]);
                }
                Action::Close { vault_id } => {
                    bytes.push(5);
                    numbers(&mut bytes, &[*vault_id]);
                }
                Action::Liquidate {
                    vault_id,
                    owner,
                    oracle_sequence,
                    max_burn,
                    min_collateral,
                } => {
                    bytes.push(6);
                    numbers(&mut bytes, &[*vault_id]);
                    bytes.extend_from_slice(&owner.0);
                    numbers(&mut bytes, &[*oracle_sequence, *max_burn, *min_collateral]);
                }
                Action::Transfer { asset, to, amount } => {
                    bytes.push(7);
                    bytes.push(match asset {
                        Asset::ZEC => 0,
                        Asset::ZUSD => 1,
                    });
                    bytes.extend_from_slice(&to.0);
                    numbers(&mut bytes, &[*amount]);
                }
            }
        }
        bytes
    }
}
impl SignedOracleReport {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = b"zsv/oracle/v2\0".to_vec();
        bytes.push(network_tag(self.network));
        bytes.extend_from_slice(&self.chain_domain);
        bytes.extend_from_slice(&self.signer.0);
        numbers(&mut bytes, &[self.sequence, self.price, self.valid_until]);
        bytes
    }
}
fn verify(key: Pubkey, message: &[u8], signature: &[u8]) -> Result<(), Error> {
    let key = VerifyingKey::from_bytes(&key.0).map_err(|_| Error::Invalid("invalid public key"))?;
    let signature =
        Signature::from_slice(signature).map_err(|_| Error::Invalid("signature length"))?;
    key.verify_strict(message, &signature)
        .map_err(|_| Error::Invalid("invalid signature"))
}
fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::ArithmeticOverflow)
}
fn sub(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_sub(b)
        .ok_or(Error::Invalid("insufficient balance"))
}
fn product(values: &[u128]) -> Result<u128, Error> {
    values.iter().try_fold(1u128, |a, b| {
        a.checked_mul(*b).ok_or(Error::ArithmeticOverflow)
    })
}
fn positive(amount: u64) -> Result<(), Error> {
    if amount == 0 {
        Err(Error::Invalid("zero amount"))
    } else {
        Ok(())
    }
}
fn safe_ratio(collateral: u64, debt: u64, price: u64, required: u64) -> Result<bool, Error> {
    if debt == 0 {
        return Ok(true);
    }
    Ok(product(&[collateral as u128, price as u128, 100])?
        >= product(&[debt as u128, ZATOSHIS_PER_ZEC as u128, required as u128])?)
}
impl State {
    /// Model-only ZEC allocation. No mainnet/testnet state or stablecoin allocation is possible.
    pub fn from_regtest_fixture(
        mut config: Config,
        allocations: impl IntoIterator<Item = (Pubkey, u64)>,
    ) -> Result<Self, Error> {
        if config.network != Network::Regtest {
            return Err(Error::Invalid("regtest only"));
        }
        let keys: BTreeSet<_> = config.oracle_keys.iter().copied().collect();
        if keys.len() != 3 {
            return Err(Error::Invalid("oracle keys must be distinct"));
        }
        for key in &keys {
            VerifyingKey::from_bytes(&key.0).map_err(|_| Error::Invalid("invalid oracle key"))?;
        }
        config.oracle_keys.sort();
        let mut accounts = BTreeMap::new();
        let mut total = 0u64;
        for (key, zec) in allocations {
            VerifyingKey::from_bytes(&key.0)
                .map_err(|_| Error::Invalid("invalid allocation key"))?;
            total = add(total, zec)?;
            if accounts
                .insert(
                    key,
                    Account {
                        zec,
                        ..Account::default()
                    },
                )
                .is_some()
            {
                return Err(Error::Invalid("duplicate fixture allocation"));
            }
        }
        // Canonical validated genesis: set-valued oracle keys and allocation entries
        // are sorted; every fixed-width integer is little-endian. This domain never
        // depends on height or mutable balances. Identical fixtures name the same chain.
        let mut domain = Sha256::new();
        domain.update(b"zsv/model-chain/v1\0");
        domain.update([network_tag(config.network)]);
        domain.update((config.oracle_keys.len() as u64).to_le_bytes());
        for key in &config.oracle_keys {
            domain.update(key.0);
        }
        domain.update((accounts.len() as u64).to_le_bytes());
        for (key, account) in &accounts {
            domain.update(key.0);
            domain.update(account.zec.to_le_bytes());
        }
        let chain_domain = domain.finalize().into();
        let state = Self {
            config,
            inner: Snapshot {
                chain_domain,
                height: 0,
                oracle: None,
                accounts,
                vaults: BTreeMap::new(),
                stable_supply: 0,
                bad_debt: 0,
                fixture_zec_total: total,
                used_vault_ids: BTreeSet::new(),
            },
        };
        state.check_invariants()?;
        Ok(state)
    }
    pub fn snapshot(&self) -> Snapshot {
        self.inner.clone()
    }
    /// Immutable canonical genesis/configuration domain, shared by forks of the same model chain.
    pub fn chain_domain(&self) -> [u8; 32] {
        self.inner.chain_domain
    }
    /// Validate ordered transactions in a private clone. Rejection never mutates the input.
    pub fn validate_block(&self, block: &Block) -> Result<Self, Error> {
        if self.config.network != Network::Regtest {
            return Err(Error::Invalid("regtest only"));
        }
        if block.height != add(self.inner.height, 1)? {
            return Err(Error::Invalid("nonconsecutive block height"));
        }
        if block.transactions.len() > MAX_TRANSACTIONS {
            return Err(Error::Invalid("too many transactions"));
        }
        let mut action_count = 0usize;
        for tx in &block.transactions {
            if tx.actions.is_empty() || tx.actions.len() > MAX_ACTIONS_PER_TX {
                return Err(Error::Invalid("invalid transaction action count"));
            }
            action_count = action_count
                .checked_add(tx.actions.len())
                .ok_or(Error::ArithmeticOverflow)?;
            if action_count > MAX_ACTIONS_PER_BLOCK {
                return Err(Error::Invalid("too many block actions"));
            }
            if tx.signature.len() != 64 {
                return Err(Error::Invalid("signature length"));
            }
        }
        let mut next = self.clone();
        next.inner.height = block.height;
        if let Some(update) = &block.oracle {
            next.apply_oracle(update)?;
        }
        for tx in &block.transactions {
            next.apply_transaction(tx)?;
        }
        next.check_invariants()?;
        Ok(next)
    }
    pub fn apply_block(&mut self, block: &Block) -> Result<(), Error> {
        *self = self.validate_block(block)?;
        Ok(())
    }
    /// Deterministic model checkpoint, including its immutable configuration.
    /// This is not a Zcash block header, consensus encoding, or network commitment.
    pub fn commitment(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"zsv/model-checkpoint/v1\0");
        hash.update(
            serde_json::to_vec(&(&self.config, &self.inner))
                .expect("model serialization is infallible"),
        );
        hash.finalize().into()
    }
    fn apply_oracle(&mut self, update: &OracleUpdate) -> Result<(), Error> {
        if !(2..=3).contains(&update.reports.len()) {
            return Err(Error::Invalid(
                "oracle quorum requires two or three reports",
            ));
        }
        let first = &update.reports[0];
        if first.price == 0 || first.sequence == 0 {
            return Err(Error::Invalid("zero oracle price or sequence"));
        }
        if self
            .inner
            .oracle
            .as_ref()
            .is_some_and(|old| first.sequence <= old.sequence)
        {
            return Err(Error::Invalid("oracle sequence must advance"));
        }
        let lifetime = first
            .valid_until
            .checked_sub(self.inner.height)
            .ok_or(Error::Invalid("expired oracle report"))?;
        if lifetime > MAX_ORACLE_LIFETIME {
            return Err(Error::Invalid("oracle expiry too far ahead"));
        }
        let mut seen = BTreeSet::new();
        for report in &update.reports {
            if report.network != Network::Regtest {
                return Err(Error::Invalid("oracle network"));
            }
            if report.chain_domain != self.chain_domain() {
                return Err(Error::Invalid("oracle chain domain"));
            }
            if (report.sequence, report.price, report.valid_until)
                != (first.sequence, first.price, first.valid_until)
            {
                return Err(Error::Invalid("inconsistent oracle reports"));
            }
            if !self.config.oracle_keys.contains(&report.signer) || !seen.insert(report.signer) {
                return Err(Error::Invalid("unregistered or duplicate oracle signer"));
            }
            verify(report.signer, &report.signing_bytes(), &report.signature)?;
        }
        self.inner.oracle = Some(Price {
            sequence: first.sequence,
            price: first.price,
            valid_until: first.valid_until,
        });
        Ok(())
    }
    fn fresh_price(&self) -> Result<u64, Error> {
        let price = self
            .inner
            .oracle
            .as_ref()
            .ok_or(Error::Invalid("oracle price missing"))?;
        if price.valid_until < self.inner.height {
            return Err(Error::Invalid("oracle price expired"));
        }
        Ok(price.price)
    }
    fn apply_transaction(&mut self, tx: &SignedTx) -> Result<(), Error> {
        if tx.network != Network::Regtest {
            return Err(Error::Invalid("transaction network"));
        }
        if tx.chain_domain != self.chain_domain() {
            return Err(Error::Invalid("transaction chain domain"));
        }
        verify(tx.signer, &tx.signing_bytes(), &tx.signature)?;
        let current = self
            .inner
            .accounts
            .get(&tx.signer)
            .map_or(0, |account| account.nonce);
        if tx.nonce != current {
            return Err(Error::Invalid("incorrect or replayed nonce"));
        }
        let next_nonce = add(current, 1)?;
        self.inner.accounts.entry(tx.signer).or_default();
        for action in &tx.actions {
            self.apply_action(tx.signer, action)?;
        }
        self.inner
            .accounts
            .get_mut(&tx.signer)
            .expect("signer account exists")
            .nonce = next_nonce;
        Ok(())
    }
    fn owned_vault(&self, id: u64, owner: Pubkey) -> Result<Vault, Error> {
        let vault = self
            .inner
            .vaults
            .get(&VaultId::new(owner, id))
            .ok_or(Error::Invalid("vault missing"))?;
        if vault.owner != owner {
            return Err(Error::Invalid("vault owner signature required"));
        }
        Ok(vault.clone())
    }
    fn debit(&mut self, key: Pubkey, asset: Asset, amount: u64) -> Result<(), Error> {
        let account = self
            .inner
            .accounts
            .get_mut(&key)
            .ok_or(Error::Invalid("account missing"))?;
        let balance = match asset {
            Asset::ZEC => &mut account.zec,
            Asset::ZUSD => &mut account.zusd,
        };
        *balance = sub(*balance, amount)?;
        Ok(())
    }
    fn credit(&mut self, key: Pubkey, asset: Asset, amount: u64) -> Result<(), Error> {
        let account = self.inner.accounts.entry(key).or_default();
        let balance = match asset {
            Asset::ZEC => &mut account.zec,
            Asset::ZUSD => &mut account.zusd,
        };
        *balance = add(*balance, amount)?;
        Ok(())
    }
    fn apply_action(&mut self, signer: Pubkey, action: &Action) -> Result<(), Error> {
        match *action {
            Action::Open {
                vault_id,
                collateral,
            } => {
                let vault_id = VaultId::new(signer, vault_id);
                positive(collateral)?;
                if self.inner.used_vault_ids.contains(&vault_id) {
                    return Err(Error::Invalid("vault id already used"));
                }
                self.debit(signer, Asset::ZEC, collateral)?;
                self.inner.vaults.insert(
                    vault_id,
                    Vault {
                        owner: signer,
                        collateral,
                        debt: 0,
                    },
                );
                self.inner.used_vault_ids.insert(vault_id);
            }
            Action::Deposit { vault_id, amount } => {
                positive(amount)?;
                let mut vault = self.owned_vault(vault_id, signer)?;
                self.debit(signer, Asset::ZEC, amount)?;
                vault.collateral = add(vault.collateral, amount)?;
                self.inner
                    .vaults
                    .insert(VaultId::new(signer, vault_id), vault);
            }
            Action::Borrow { vault_id, amount } => {
                positive(amount)?;
                let mut vault = self.owned_vault(vault_id, signer)?;
                if self.inner.bad_debt > 0 {
                    return Err(Error::Invalid("borrowing frozen by bad debt"));
                }
                let price = self.fresh_price()?;
                vault.debt = add(vault.debt, amount)?;
                if vault.debt < MIN_DEBT {
                    return Err(Error::Invalid("debt below minimum"));
                }
                if !safe_ratio(vault.collateral, vault.debt, price, 250)? {
                    return Err(Error::Invalid("opening collateral ratio below 250 percent"));
                }
                let supply = add(self.inner.stable_supply, amount)?;
                if supply > SUPPLY_CAP {
                    return Err(Error::Invalid("stablecoin supply cap"));
                }
                self.credit(signer, Asset::ZUSD, amount)?;
                self.inner.stable_supply = supply;
                self.inner
                    .vaults
                    .insert(VaultId::new(signer, vault_id), vault);
            }
            Action::Repay { vault_id, amount } => {
                positive(amount)?;
                let mut vault = self.owned_vault(vault_id, signer)?;
                vault.debt = sub(vault.debt, amount)?;
                if vault.debt != 0 && vault.debt < MIN_DEBT {
                    return Err(Error::Invalid("remaining debt below minimum"));
                }
                self.debit(signer, Asset::ZUSD, amount)?;
                self.inner.stable_supply = sub(self.inner.stable_supply, amount)?;
                self.inner
                    .vaults
                    .insert(VaultId::new(signer, vault_id), vault);
            }
            Action::Withdraw { vault_id, amount } => {
                positive(amount)?;
                let mut vault = self.owned_vault(vault_id, signer)?;
                vault.collateral = sub(vault.collateral, amount)?;
                if vault.debt > 0
                    && !safe_ratio(vault.collateral, vault.debt, self.fresh_price()?, 250)?
                {
                    return Err(Error::Invalid(
                        "withdrawal collateral ratio below 250 percent",
                    ));
                }
                self.credit(signer, Asset::ZEC, amount)?;
                self.inner
                    .vaults
                    .insert(VaultId::new(signer, vault_id), vault);
            }
            Action::Close { vault_id } => {
                let vault = self.owned_vault(vault_id, signer)?;
                if vault.debt != 0 {
                    return Err(Error::Invalid("repay before close"));
                }
                self.credit(signer, Asset::ZEC, vault.collateral)?;
                self.inner.vaults.remove(&VaultId::new(signer, vault_id));
            }
            Action::Liquidate {
                vault_id,
                owner,
                oracle_sequence,
                max_burn,
                min_collateral,
            } => {
                let vault_id = VaultId::new(owner, vault_id);
                let vault = self
                    .inner
                    .vaults
                    .get(&vault_id)
                    .ok_or(Error::Invalid("vault missing"))?
                    .clone();
                let price = self.fresh_price()?;
                if self.inner.oracle.as_ref().map(|p| p.sequence) != Some(oracle_sequence) {
                    return Err(Error::Invalid("liquidation oracle sequence changed"));
                }
                if safe_ratio(vault.collateral, vault.debt, price, 200)? {
                    return Err(Error::Invalid("vault not liquidatable"));
                }
                let capacity = product(&[vault.collateral as u128, price as u128, 100])?
                    / product(&[ZATOSHIS_PER_ZEC as u128, 105])?;
                let burn = (vault.debt as u128).min(capacity) as u64;
                let numerator = product(&[burn as u128, ZATOSHIS_PER_ZEC as u128, 105])?;
                let denominator = product(&[price as u128, 100])?;
                let quotient = numerator / denominator;
                let seized = u64::try_from(quotient + u128::from(numerator % denominator != 0))
                    .map_err(|_| Error::ArithmeticOverflow)?;
                if seized > vault.collateral {
                    return Err(Error::InvariantViolation);
                }
                if burn > max_burn || seized < min_collateral {
                    return Err(Error::Invalid("liquidation exceeds signed intent bounds"));
                }
                self.debit(signer, Asset::ZUSD, burn)?;
                self.credit(signer, Asset::ZEC, seized)?;
                self.credit(vault.owner, Asset::ZEC, sub(vault.collateral, seized)?)?;
                self.inner.stable_supply = sub(self.inner.stable_supply, burn)?;
                self.inner.bad_debt = add(self.inner.bad_debt, sub(vault.debt, burn)?)?;
                self.inner.vaults.remove(&vault_id);
            }
            Action::Transfer { asset, to, amount } => {
                positive(amount)?;
                VerifyingKey::from_bytes(&to.0)
                    .map_err(|_| Error::Invalid("invalid recipient key"))?;
                self.debit(signer, asset, amount)?;
                self.credit(to, asset, amount)?;
            }
        }
        Ok(())
    }
    /// Conservation checks over every accepted account and vault.
    pub fn check_invariants(&self) -> Result<(), Error> {
        let mut zec = 0u64;
        let mut stable = 0u64;
        let mut debt = self.inner.bad_debt;
        for account in self.inner.accounts.values() {
            zec = add(zec, account.zec)?;
            stable = add(stable, account.zusd)?;
        }
        for (id, vault) in &self.inner.vaults {
            if !self.inner.used_vault_ids.contains(id)
                || !self.inner.accounts.contains_key(&vault.owner)
                || id.owner != vault.owner
            {
                return Err(Error::InvariantViolation);
            }
            if vault.debt != 0 && vault.debt < MIN_DEBT {
                return Err(Error::InvariantViolation);
            }
            zec = add(zec, vault.collateral)?;
            debt = add(debt, vault.debt)?;
        }
        if zec != self.inner.fixture_zec_total
            || stable != self.inner.stable_supply
            || debt != self.inner.stable_supply
            || stable > SUPPLY_CAP
        {
            return Err(Error::InvariantViolation);
        }
        Ok(())
    }
}
