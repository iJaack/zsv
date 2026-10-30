# Transparent collateral development profile

The [collateral runner](../zebra/dev-harness/src/collateral.rs) connects the signed vault model to serialized ZEC outputs. It resolves actual previous outputs, verifies wallet signatures and checks every vault spend against the model transition. Every wallet and vault must have exactly the ZEC recorded in its native UTXOs after each transaction.

This is an offline experiment linked to `zebra-chain` 15.0.0 at `5faf8a413f0d30a6e93115f3d16af1e229fd7858`. It does not patch Zebra's node verifier. Its vault script ends in `OP_DROP OP_TRUE`: unmodified Zcash nodes would allow anyone to spend it. Only the experimental runner enforces the vault restriction. All funding and keys are public fixtures; zUSD remains a model balance.

## Bootstrap and ownership

`CollateralHarness::new` accepts only default isolated Regtest parameters and a vault-only model at height zero. Reserve-enabled state is rejected even when its funding schedule would pay nothing. Native reserve funding and shutdown require a separate cash path before they can enter this profile.

Each model account has one fixed Ed25519 owner key and a distinct compressed secp256k1 key. The constructor requires a bijection between model accounts and native keys. Ownership cannot be inferred by reinterpreting bytes between the two key schemes. Registration and key rotation are not implemented.

A canonical V4 funding coinbase supplies P2PK outputs matching the model's initial free ZEC exactly. Total fixture issuance must fit Zebra's `Amount<NonNegative>` range, at most 21 million ZEC. A custom bootstrap block includes that transaction and its computed Merkle root. It uses a Regtest header template, a zero previous hash and nonce zero. It is unmined, with trusted issuance and no subsidy or maturity check.

The bootstrap's native height is 1 and the model's is 0. Later native heights equal model height plus one. The root hash is the custom bootstrap hash, not standard Regtest genesis. The profile domain hashes the model genesis checkpoint, sorted owner mappings and complete serialized bootstrap block. Changing native allocations, their outpoints or the ownership mapping therefore changes the domain.

## Transaction and signature rules

The [wire module](../zebra/dev-harness/src/collateral_wire.rs) permits transparent V4 transactions with no shielded data, lock time zero, final input sequences and `1 <= expiry < 500,000,000`. A transaction expires when its expiry height is less than the candidate native block height. Fees are zero: actual input value must equal output value.

Only three exact output scripts are recognized:

| Output | Contents | Authorization |
|---|---|---|
| Free ZEC | Compressed secp256k1 key followed by `OP_CHECKSIG` | Native signature and matching model owner |
| Vault ZEC | `ZSVLOCK1`, Ed25519 owner and local vault ID, then `OP_DROP OP_TRUE` | Mandatory contextual vault guard |
| Commitment | `OP_RETURN`, `ZSVCOL01` and model-request digest | Exactly one, with zero value |

Wallet inputs require a minimal push of a strict DER, low-S ECDSA signature with `SIGHASH_ALL`. Verification uses Zebra's ZIP-243 signature hash with a fixed Sapling branch context and the actual branch prevout value and script. This historical context is part of the experiment; it does not follow current Regtest activation rules. The checker implements this restricted P2PK grammar, not a general Zcash script interpreter. Vault input scripts must be empty.

The model request retains its existing Ed25519 signature. A second Ed25519 native intent signs the domain separator `zsv/native-collateral/intent/v1`, the collateral profile domain, exact parent profile checkpoint, unsigned native transaction bytes and complete signed model request. Native input scripts are cleared for this intent; their signatures are checked separately. Native intents prevent a valid model request from authorizing arbitrary native outputs or a different parent state. Model and oracle signatures retain the original model genesis domain; only the native intent adds the collateral profile binding.

Every native user transaction needs one signed model request and one native intent. Even a stablecoin-only transfer needs a native input, which may be a zero-value wallet output. Unannotated native transactions are rejected. There is no general token issuance or asset-registration interface.

## State transition

The envelope contains the `ZSVCOL01` version marker, exact parent checkpoint, length-delimited native block and canonical JSON model payload. Parsing rejects trailing bytes and noncanonical reserialization. The runner then:

1. Resolves the native parent hash to its stored branch state and verifies the supplied checkpoint.
2. Checks the native height, exact zero-valued commitment coinbase, transaction Merkle root, unique transaction IDs and one-to-one native/model pairing.
3. Validates the complete model block using `State::validate_block_with_trace`. The returned snapshots cover the block prelude and each ordered transaction; rejected blocks return no trace.
4. Reconciles the prelude, then resolves every native input from the candidate branch UTXO set. Missing, duplicate and previously spent inputs fail.
5. Requires each wallet input to belong to the model signer. Every vault input and output must identify a vault touched by an authorized action, including a replacement with no balance change.
6. Checks native signatures, expiry, intent, model commitment and zero-fee value conservation. Applies the native inputs and outputs to the candidate state and reconciles it with that transaction's model snapshot.
7. Commits the model and UTXO state together only after all transactions pass.

For each account a and vault v, reconciliation requires:

```text
sum(native wallet outputs for a) = model free ZEC[a]
sum(native vault outputs for v)  = model collateral[v]
sum(all spendable native outputs) = fixed fixture issuance
```

Unknown owners, unknown scripts and outputs naming nonexistent vaults fail. Comparing each bucket prevents substituting one owner's backing for another's. Checking after every transaction prevents temporarily spending collateral without a valid transition and restoring it at block end. Same-block spends of earlier transaction outputs remain valid when all rules pass.

Branches hold separate model and UTXO states. Validation uses the native parent, independently of the manually selected tip. Checkpoints bind the profile domain, model checkpoint, sorted outpoints, output bytes, creation heights and coinbase flags. Returned states and UTXO lists are detached copies. The new profile has no persistent database or journal import; its scenarios replay saved input envelopes from the trusted bootstrap.

## Reproduction and observed examples

```sh
bash zebra/verify-dev.sh
cargo run --locked --manifest-path zebra/dev-harness/Cargo.toml --bin collateral_scenarios
```

The [checked-in output](../zebra/dev-harness/collateral-scenario-output.json) contains the bootstrap, ownership bindings, serialized envelopes and checkpoints. Three scenarios begin with 20 ZEC each for the borrower and liquidator. The borrower locks 10 ZEC and issues 400 zUSD at a 100-dollar ZEC price.

| Scenario | Result |
|---|---|
| Repay and close | Burns 400 zUSD and returns all 10 ZEC collateral |
| Liquidate at 50 dollars | Burns 400 zUSD; pays 8.4 ZEC to liquidator and 1.6 ZEC to owner |
| Liquidate at 20 dollars | Burns 190.476190 zUSD; pays 9.99999998 ZEC to liquidator and 2 zatoshi to owner; records 209.523810 zUSD bad debt |

All three conserve 40 ZEC. Replay checks model and native checkpoints. The [independent review](collateral-review.md) records attack tests, reproduced defects and remaining weaknesses; the [verification receipt](verification.md) records the full checks.

## Limits and next integration

The runner limits a transaction to 64 inputs, 64 outputs and 65,536 bytes; an envelope to 3,048,624 bytes; its native block to 2,000,000 bytes; and its JSON payload to 1,048,576 bytes. It retains at most 4,096 UTXOs and 256 blocks after bootstrap. These are finite-test limits. Zero fees and zero-value outputs permit an actor to occupy the UTXO allowance, so they do not establish production denial-of-service resistance.

The profile proves neither a market peg nor complete Zcash consensus. It omits native stablecoin encoding/value balance, shielded proofs, current activation rules, PoW, difficulty, mining rewards, coinbase maturity, fee accounting, consensus fork choice, durable branch storage and node/network services. It shares the model's oracle and economic risks. Reserve accounting remains available only in the separate model and older block-commitment runner.

At the pinned Zebra source, `zebra-state/src/service/check/utxo.rs::transparent_spend` resolves every transparent input against the selected parent chain. A native guard must run after that resolution and before committing the spend. It must also protect `zebra-state/src/service/finalized_state.rs::commit_finalized_direct`, whose checkpoint path can bypass ordinary non-finalized checks. Mempool checks must agree, and native semantic/script/proof verification remains required. Vault state must follow Zebra's branch and finalized database state atomically. These changes, a native stablecoin pool and restart/reorg tests are the next implementation milestone; see the [integration proposal](zebra-integration.md).
