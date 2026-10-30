# Offline Zebra-linked development harness

This crate compiles against Zebra `5faf8a413f0d30a6e93115f3d16af1e229fd7858`. It uses Zebra's native block/transaction decoder, coinbase-height parser, transaction Merkle root, block hash and network parameters. The signed ZSV ledger runs against the exact parent selected by that native block hash.

No Zebra node service is patched or launched. The fixture transactions carry a data commitment, not a collateral lock or native stablecoin. The harness does not verify proof of work, transaction scripts, subsidy amounts, upgrade activation or shielded proofs. Its historical V1 coinbases are codec fixtures; successful parsing does not establish consensus validity under current Regtest rules.

## Run

Requires Rust 1.90 and Cargo nextest. The verified nextest version is 0.9.100:

```sh
cargo install cargo-nextest --locked --version 0.9.100
bash zebra/verify-dev.sh
```

Run these commands from the repository root. The script checks the harness with nextest, rustfmt, strict Clippy and an exact scenario-output comparison. The separate lockfile pins its dependency resolution, including the full Zebra Git revision. Initial dependency downloads require network access. The default build directory is the root's ignored `target/zebra-dev`; set `CARGO_TARGET_DIR` to use another cache.

```sh
cargo run --locked --manifest-path zebra/dev-harness/Cargo.toml --bin scenarios
```

The executable carries all three checked-in signed reserve scenarios through native block commitments, then exports and replays their input journals. It checks the same model checkpoints as the original scenarios. [scenario-output.json](scenario-output.json) records native tip hashes, journal hashes and model checkpoints.

## Native commitment

Each accepted fixture has exactly one V1 coinbase transaction, one coinbase input and one zero-value output. Its script is exactly `OP_RETURN`, a canonical 40-byte push, `ZSVDEV01`, then a 32-byte SHA-256 digest. The digest covers, in order:

1. The bytes `zsv/zebra-dev/commitment/v1` followed by a zero byte.
2. The immutable model chain domain and exact parent model checkpoint, each 32 bytes.
3. The canonical payload length as little-endian u64, then its JSON bytes.

The payload is the existing signed `zsv::Block`. Decoding must reproduce exactly the same bytes when serialized; alternate whitespace, reordered fields, duplicate fields and unknown fields fail. Its signed transactions and oracle reports are verified by the ledger after native context checks.

The harness recomputes the transaction Merkle root and compares it with the native header. The V1 transaction hash binds the full coinbase bytes, including the commitment. This gives the payload a real native-header commitment; supplying different valid model data beside an unchanged header fails. Restricting the block to one V1 coinbase also excludes duplicate-transaction Merkle ambiguity and V5 authorization data that is absent from the transaction ID.

The block's previous hash selects an already accepted parent. Coinbase height must equal model height, which the ledger requires to be exactly parent height plus one. The parent checkpoint in the envelope must equal that parent's stored checkpoint. All these checks and the signed model transition finish before the branch archive changes.

The native root is Zebra's fixed Regtest genesis block. The caller supplies a trusted model genesis at height zero. Only the exact default Zebra Regtest parameters are accepted; Mainnet, public Testnet and alternate parameter sets fail. Native blocks carry no network tag of their own, so the constructor and fixed root establish this experimental context.

## Bounded envelope and archive

The binary envelope is `ZSVDEV01`, the 32-byte parent model checkpoint, a little-endian u32 native-block length and its bytes, then a little-endian u32 payload length and its canonical JSON bytes. Every slice length is checked before parsing. Trailing native, payload or envelope bytes fail.

| Resource | Maximum |
|---|---:|
| Native serialized block | 2,000,000 bytes |
| Model payload | 1,048,576 bytes |
| Complete envelope | 3,048,624 bytes |
| Accepted blocks across all branches | 256 |
| Complete replay journal | 16,777,216 bytes |

The archive retains a separate complete model state for every accepted native block hash. `accept` can extend any known parent and selects the accepted child as the inspected tip. `select_tip` selects an existing branch explicitly. There is no chain-work comparison, mining or automatic fork-choice policy. The bounds belong to this offline experiment, not a production resource-cost model.

A journal records the model chain domain, genesis checkpoint, selected native tip and accepted envelopes in insertion order. It never imports asserted post-state balances. `Harness::replay` requires the original model genesis and a separately trusted `JournalAnchor`, containing both the selected tip and SHA-256 of the complete journal. The external digest detects deletion of an unselected sibling as well as selected-chain truncation. A digest stored only inside an attacker-controlled journal would provide no such authentication.

Replay verifies the external digest and all envelope/native/model rules again, reconstructing every stored branch. This is input replay and exact archive comparison. It does not test disk atomicity, crash recovery, finalization or database migrations.

## Remaining native work

A deployable ZSV extension still needs a specified transaction bundle and native asset/value-pool rules below Zebra, native ZEC collateral locks, mint/burn proof binding, semantic and parent-branch verification, persistent state and mempool integration. The approved funding schedule and native NSM accounting are also absent. The coinbase marker here supplies evidence about data binding and branch selection only.

Zebra remains an external pinned dependency under its MIT/Apache-2.0 licenses. The harness invokes its Regtest genesis helper directly; it does not copy or relicense upstream source or fixtures. See the [source register](../../docs/sources.md) and [native implementation proposal](../../docs/zebra-integration.md).
