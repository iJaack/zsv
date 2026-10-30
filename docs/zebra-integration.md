# Native ZSV integration with Zebra

Status: native enforcement proposal with offline block and collateral profiles linked to Zebra; no deployed consensus change. The collateral profile verifies serialized fixture UTXOs and restricted native signatures, but is not installed in Zebra's node verifier. ZSV (Zcash Stablecoin Vault) would lock ZEC in protocol-controlled vaults and mint one protocol-defined stablecoin against that collateral. Zcash nodes must validate all ownership, mint, burn, withdrawal, liquidation and state-transition rules. General asset creation, issuer keys, arbitrary metadata, user-selected tickers, arbitrary base points and external settlement chains are excluded.

The immutable source baseline is Zebra `5faf8a413f0d30a6e93115f3d16af1e229fd7858` (2026-10-28) and ZIPs `2bdf39494320e5e04057ac93b1794031ebfb00b5` (2026-10-21), researched 2026-10-29. See [sources.md](sources.md) for verified links, dates, status and evidence limits.

## Candidate specification now governs implementation

The [ZIP candidate](../zips/draft-ijaack-zsv.md) selects a dedicated future transaction format, public zUSD UTXOs and a separate native ZEC vault value pool. Its normalized operations, 5-of-7 oracle, domains and bad-debt retirement are proposed rules beyond the existing harness. It has no trusted bootstrap issuance or OP_TRUE collateral UTXO. Native implementation should follow that specification; the earlier API sketches and experimental guard below remain source-mapping and research evidence.

Production transaction hashing will require trusted network context because the candidate commits the network genesis hash in addition to its serialized branch ID. The existing wrapper API does not supply that change automatically. Library and node work must add the parser/digests, stable UTXO namespace, vault-pool accounting, exact-parent atomic state transitions, auth-commitment/relay dispatch and all finalization paths. Merely installing the prototype marker guard would not implement this ZIP.

## What can be reused

ZSV could reuse ZSA cryptography for moving a non-ZEC asset. Permissionless issuance remains excluded.

- Study ZIP 226's asset-bound note commitments, value commitments, nullifiers, transfer conservation, burn accounting and split-note constraints.
- Fix the asset domain to exactly ZEC and one canonical ZSV unit in the consensus implementation and circuit. An enum in a wallet does not enforce this: the proof must constrain the asset/base to a permitted constant, or the public typed-asset validator must reject every other value.
- Mint only when the native vault transition increases debt by the same amount. Burn only when actual stablecoin inputs are consumed. Each approved mint or burn amount must be tied to the same transaction's stablecoin output/input commitments and public supply delta.
- Omit ZIP 227's arbitrary issuer authorization keys, user-created identifiers, issuance registry, NFT series, arbitrary metadata, configurable supply series and general issuance bundles.
- Borrow existing Orchard/Ironwood proof infrastructure and wallet concepts only after reconciling their current circuits and formats. ZIP 226 is explicitly defined relative to ZIP 2005 changes. Reusing cryptographic code does not make the new constraints audited.

A fixed set of asset constants simplifies valid-base checks; it does not justify accepting arbitrary curve points or weakening note provenance. ZIP 226 documents how unrestricted asset bases could counterfeit ZEC.

## Current protocol constraints

ZIP 222 (TZEs) is still Draft. Its architecture of consensus-distributed, finite extension modules is relevant to a vault covenant, but there is no verified production TZE entry point to register a stablecoin today. A TZE-based route would itself require specification, deployment, digest changes and a fixed ZSV extension type. It cannot be assumed available as a plug-in.

ZIP 230 is Withdrawn. Do not implement the historical ZSA “v6” format. ZIP 229 now defines v6 for NU6.3/Ironwood, with version group ID `0xD884B698`; it explicitly does not require ZSAs. ZIP 259 says NU7 introduces no new format and its listed deployment scope excludes ZSA ZIPs 226/227. Some draft ZSA text still refers to the withdrawn v6 proposal.

ZSV therefore needs a separately reviewed transaction-format extension and activation proposal. “ZsvVNext” is a development placeholder, not a reserved version, branch ID or asset identifier. Choose actual numbers through the Zcash upgrade process. Existing NU6.3/NU7 constants in the sources are evidence about upstream, not ZSV activation parameters.

This cannot be deployed as a wallet policy or a mempool-only patch: miners could bypass either. An OP_RETURN claim does not make collateral encumbered and does not make a stablecoin native.

## Exact API baseline

At the Zebra pin, `zebra_chain::transaction::Transaction` is a struct wrapping `zcash_primitives::transaction::Transaction`. Its public accessors include:

```rust
pub fn tx_version(&self) -> TxVersion;
pub fn version(&self) -> u32;
pub fn network_upgrade(&self) -> Option<NetworkUpgrade>;
pub fn version_group_id(&self) -> Option<u32>;
pub fn inputs(&self) -> Vec<transparent::Input>;
pub fn outputs(&self) -> Vec<transparent::Output>;
pub fn spent_outpoints(&self) -> impl Iterator<Item = transparent::OutPoint> + '_;
pub fn hash(&self) -> transaction::Hash;
pub fn auth_digest(&self) -> Option<AuthDigest>;
```

These methods exist at the upstream pin. No `zsv_bundle()` accessor was observed. The inner transaction accessor is crate-private. Do not write an adapter that assumes the older `Transaction::V5` enum.

The semantic services in `zebra-consensus/src/transaction.rs` use:

```rust
pub struct BlockRequest {
    pub transaction_hash: transaction::Hash,
    pub transaction: Arc<Transaction>,
    pub known_utxos: Arc<HashMap<transparent::OutPoint, transparent::OrderedUtxo>>,
    pub height: block::Height,
    pub time: DateTime<Utc>,
}
pub struct MempoolRequest {
    pub transaction: UnminedTx,
    pub height: block::Height,
}
```

Shared `check_common_consensus_rules(tx, height, network)` at line 791 calls `check_structure_and_network_rules` at line 819. The version dispatcher at line 953 recognizes V4/V5/V6 and rejects unsupported variants. Both block and mempool verification need ZSV structural checks and proof verification.

Production transaction serialization lives below Zebra: `Transaction::zcash_serialize` calls `self.0.write`, and parsing calls `zp_tx::Transaction::read`. `transaction/serialize.rs` explicitly explains that most local shielded codecs are test-only. The dependency baseline is `zcash_primitives 0.31.0`, `zcash_protocol 0.11.0`, `orchard 0.16.0`; immutable registry checksums are in the source register.

## Required changes, in dependency order

| Layer | Concrete source location | Required ZSV work |
|---|---|---|
| Native transaction / digest | Pinned `zcash_primitives` dependency, then `zebra-chain/src/transaction.rs` and `transaction/compat.rs` | Add a bounded, explicitly versioned ZSV bundle and typed stablecoin transfer representation. Commit all non-authorizing data in txid; commit proofs/signatures in auth data and bind authorization to branch, version, bundle and relevant state. Reject unknown tags and noncanonical encodings. |
| Asset/circuit semantics | New reviewed proof code; `zebra-consensus/src/primitives/halo2.rs` integration | Fixed asset domains; proof of ownership and conservation; public mint/burn delta binding; vault transition commitments; separate verification key/version/cache domain. Existing Orchard proofs prove ZEC transfers and do not prove these new statements. |
| Common semantic checks | `zebra-consensus/src/transaction.rs` common checks and version dispatch | Reject unsupported network/epoch/version and malformed bundles; enforce counts/size limits and no ZSV in coinbase; verify real signatures and proofs. The standard ZEC checks remain necessary. |
| Contextual block execution | `zebra-state/src/service/write.rs::validate_and_commit_non_finalized`; `service/non_finalized_state.rs::commit_block` | Resolve the exact parent branch's vault/oracle/supply state; execute transitions in transaction order against a temporary overlay; reject conflicting operations; apply all deltas atomically only after every check succeeds. |
| All-input collateral guard | `zebra-state/src/service/check/utxo.rs::transparent_spend` | After resolving each prevout on the exact parent branch, require an authorized vault transition for any encumbered output. Check ordinary transactions without a ZSV annotation as well. Protect or explicitly fence checkpoint/direct-finalization paths that bypass this check. |
| Fork state and rollback | `service/non_finalized_state/chain.rs`: `push`, `fork`, `pop_root`, tip updates; `request.rs` verified-block / treestate types | Carry vaults, debt, issued supply, stablecoin UTXOs/nullifiers, oracle epoch and settlement status per branch. Clone/fork and reverse transitions exactly. Never validate a side branch against the current best-chain state. |
| Finalized storage | `service/finalized_state/column_family.rs`, `zebra_db.rs`, disk formats and database version | Add indexed durable state and atomic write batches; migration/replay logic; preserve restart and finalization equivalence with non-finalized execution. |
| Mempool / block templates | Transaction mempool verifier; `zebrad/src/components/mempool/storage.rs` and `storage/verified_set.rs` | State-aware conflicts, dependencies, mint budgets and oracle epochs; revalidate on tip/oracle/activation changes. Admission is provisional; block execution remains authoritative. |
| Transport / policy | `zebra-network/src/protocol/external/codec.rs`; transaction size and action-cost paths | Bound all allocations and cryptographic work before accepting untrusted bundles; update fee/cost accounting and peer protocol compatibility where required. |
| Upgrade and isolated testing | `zebra-chain/src/parameters/network_upgrade.rs`, `parameters/constants.rs`, testnet parameters | Allocate a distinct experimental network identity and separate activation in isolated tests. No Mainnet or public-Testnet acceptance follows from a local flag. Full production activation remains undefined. |

The stablecoin transition engine should be a synchronous, deterministic library, independent of network requests and Zebra service orchestration. Zebra owns parsing, cryptographic verification, state access and commit/rollback. An external process that reports “valid” cannot replace consensus execution.

## Compiled contextual seam in this repository

[`zebra/contextual_adapter.rs`](../zebra/contextual_adapter.rs) is an executable proposal using this repository's real `zsv::{State, Block, Snapshot}` API. It does not import Zebra or assume an upstream `zsv_bundle()` method. The integration test includes the module by path; the seam has no production node activation path.

`validate(Context, &State, &Block)` requires Regtest, the expected parent checkpoint, and matching consecutive context/block heights. It calls `State::validate_block`, which checks the model's actual Ed25519 transaction and oracle signatures and executes its ordered operations. It returns a `PreparedTransition` with private fields and no `Clone` or deserialization implementation. A detached `Snapshot` permits inspection without candidate mutation.

`State::validate_block_with_trace` uses the same execution path and additionally returns detached snapshots after the block prelude and each transaction. It returns a trace only when the complete block validates. The collateral profile uses these snapshots for per-transaction reconciliation without repeatedly executing prefixes; the original signing and checkpoint encodings remain unchanged.

Model transaction and oracle signatures also bind an immutable `State::chain_domain()`. The initializer derives this SHA-256 domain from `zsv/model-chain/v1\0`, the Regtest network tag, the validated and sorted oracle keys, and sorted genesis account allocations, with fixed-width counts and amounts. `Snapshot.chain_domain` retains it across blocks and forks; changing configuration or fixture allocations produces a different signing domain. Identical canonical configurations and allocations intentionally identify the same model chain. This is distinct from the changing state checkpoint and from a future native Zcash consensus-branch signature domain.

The reserve-enabled constructor uses a version-two chain domain that additionally binds the original genesis domain, every reserve parameter, the named initial donor and its ZEC contribution. Its snapshot includes reserve balances, funding counters and lifecycle state in the same checkpoint. The original constructor, signing bytes and vault-only scenario remain a separate unchanged baseline. Both constructors reject Mainnet and public Testnet.

`commit(PreparedTransition, &mut State)` consumes that preparation, rechecks the exact parent checkpoint and height, and replaces the state once. The exclusive mutable borrow covers both check and replacement. Sibling preparations or duplicate preparations become stale after the first commit; a failed preparation or commit leaves its input unchanged. This is in-memory atomic replacement, not durable database atomicity.

The SHA-256 value returned by `State::commitment()` is a deterministic model checkpoint, including model configuration. It is not a ZIP 244 digest, Zcash block hash or specified protocol state root. It also does not authenticate the caller's choice of chain branch. A native integration must obtain the candidate block's parent branch inside `zebra-state`, keep the corresponding ZSV state with that branch, and bind its transition to the native transaction/signature and collateral-spend rules.

In the pinned source, that future call belongs in the parent-specific contextual execution reached through `service/write.rs::validate_and_commit_non_finalized` and `service/non_finalized_state.rs::commit_block`, before the branch is accepted. Candidate ZSV changes must remain private until all ordinary and ZSV block checks pass; the actual branch/database commit must atomically include both sets of changes and their rollback data. A separate best-chain lookup or a commit in only the mempool verifier would be insufficient.

[`tests/zebra_adapter.rs`](../tests/zebra_adapter.rs) covers preparation/commit, parent and network mismatch, nonconsecutive or inconsistent heights, stale siblings and replay, commit onto another state at the same height, late transaction failure, detached inspection and signature tampering. These tests compile the seam against the local crate; they are not patched-Zebra node tests. A native serialized bundle, branch/version-aware signing, ZEC outpoint locking, stablecoin transfer encoding, durable state and full-node activation remain required.

## Zebra-linked block development

The separate [development runner](../zebra/dev-harness) imports `zebra-chain` from the exact pin above. It uses Zebra's native block/transaction decoder, block hash, transaction Merkle root, coinbase-height extraction and regtest parameters. It does not patch a node service or enable a new transaction format. [zebra-development.md](zebra-development.md) describes the experiment and its trust boundary.

Each fixture contains one V1 coinbase transaction with one zero-valued output. Its exact `OP_RETURN` script commits a version tag and SHA-256 digest of the model chain domain, parent model checkpoint and canonical signed payload. The runner checks the output and native Merkle root, so the native header hash binds that payload. Restricting the carrier to one V1 transaction avoids treating V5's non-authorizing txid as a commitment to all transaction bytes. A data commitment does not lock collateral or create native zUSD.

Only the default Zebra regtest parameters are accepted, with the native root fixed to regtest genesis and the model root supplied as trusted configuration. Native block bytes contain no network tag. Accepted children resolve their exact parent by native block hash and re-execute the signed ledger before entering an in-memory branch archive. Selected-tip changes are explicit experiments without a proof-of-work fork-choice rule.

The envelope caps native bytes at 2,000,000 and canonical model payloads at 1 MiB before parsing. The archive permits at most 256 non-genesis blocks; journals are capped at 16 MiB. Journal replay requires trusted genesis and an external receipt containing the selected tip and complete journal digest, then repeats validation rather than importing post-state. These bounds limit the offline experiment; they do not establish production peer resource costs or crash-safe persistence.

The fixtures are not mined or fully consensus-verified. Equihash, difficulty, ordinary input/script validation, subsidies and shielded proofs remain outside the runner. Native ZEC funds and the model's synthetic funding are separate. The [development review](development-review.md) and [verification receipt](verification.md) record the observed test scope.

## Transparent collateral profile

[`collateral.rs`](../zebra/dev-harness/src/collateral.rs) and [`collateral_wire.rs`](../zebra/dev-harness/src/collateral_wire.rs) add a distinct vault-only experiment. `CollateralHarness::new` accepts only default isolated Regtest parameters, a height-zero model state, explicit model/native owner bindings and a fixture funding transaction. The custom bootstrap block contains that funding coinbase at native height 1 and computes its transaction Merkle root and block hash. Initial native outputs must exactly match model allocations. Its zero previous hash does not make it the standard Regtest genesis or a consensus-valid block. Funding is trusted fixture issuance without subsidy, maturity or mining checks.

The profile domain commits the complete bootstrap block, sorted owner bindings and model genesis commitment. Each branch checkpoint also commits its complete UTXO set and model checkpoint. Child blocks use native height `model height + 1` and resolve state from their actual native parent hash. Candidate model and UTXO changes remain private until every transaction passes; selecting a sibling selects both states together. There is no durable replay journal for this profile.

Every user transaction is restricted to transparent V4 in a fixed historical Sapling signing context. It requires finite positive expiry below 500,000,000 and must not have expired at the candidate native height; zero expiry is rejected. Free outputs use compressed secp256k1 P2PK keys. Signatures must use canonical DER, low-S and exactly `SIGHASH_ALL`; digests use the actual branch prevouts. This is a bounded signature verifier, not a general Zcash script interpreter. The fixture binds each Ed25519 model key to a distinct native key. A separate Ed25519 intent binds the exact unsigned native transaction, signed model request, profile domain and parent checkpoint.

All native user transactions require a paired model request and at least one native input. The guard checks every input, rejects duplicate or missing prevouts and requires an authorized action for every vault input and output, even when its ZEC total does not change. After each transaction, native wallet and vault totals must equal the corresponding validated model snapshot. A zero-valued data output commits the model request; its presence alone authorizes nothing. Inputs and outputs conserve ZEC exactly, with zero fees. Reserve-enabled genesis, synthetic funding, additional coinbases, shielded paths and other formats are rejected. zUSD remains in the signed model ledger.

The experimental vault script ends in `OP_DROP OP_TRUE`. An unmodified node would allow that script to be spent; only this profile's guard applies the vault restriction. The profile accepts neither Mainnet nor Testnet configuration, and exposes no node service or real-fund deposit path. [collateral-profile.md](collateral-profile.md) records its encoding, limits and tests.

### Installing the guard in Zebra

At the pinned source, the ordinary transparent-spend boundary is `zebra-state/src/service/check/utxo.rs::transparent_spend`. A future guard must inspect the prevout resolved by `transparent_spend_chain_order` before inserting it into the block's spend set. The caller must supply the same parent-specific state used by `NonFinalizedState::validate_and_commit`; a lookup against the selected tip would be wrong for a sibling branch.

The node must check encumbered inputs in every accepted transaction version, including ordinary transactions with no ZSV bundle and transactions that send value into a shielded pool. The prototype's V4-only rejection is a local scope restriction, not a complete node policy. Native verification must retain the ordinary script, proof, expiry, fee and value-conservation checks.

The checkpoint path can reach `commit_finalized_direct` without ordinary non-finalized contextual execution. It must enforce equivalent vault checks or be explicitly fenced from the activated ZSV range. Adding a guard only to the mempool or non-finalized path leaves a consensus bypass. Branch overlays, finalization and durable write batches must carry the same vault encumbrances and stablecoin liabilities. These are proposed changes; no Zebra service is patched here.

## Validation contract for a real adapter

The following is a proposed contract, not an existing upstream API:

```text
parse_native_bundle(tx) -> bounded, canonical ZsvBundle or None
check_semantics(bundle, verified_transaction_context) -> VerifiedZsvOperation
apply_block(parent_branch_state, ordered_verified_operations)
    -> candidate_state + reversible_delta, or consensus error
commit(block, candidate_state, delta) -> atomic branch update
```

`VerifiedZsvOperation` must only be constructible after signature/proof validation. Passing a JSON field such as `proof_valid: true`, a caller-provided balance, or an arbitrary state delta is not verification.

A native bundle must bind network/branch, transaction version, vault identifier, spent collateral outpoints, stablecoin inputs/outputs, operation amounts, owner authorization, oracle epoch, expiry and any required state-root commitment. Include replay protection and canonical integer encodings. Validate parent-state membership and unspentness independently.

If a prototype instead exposes only `evaluate(state, operation)`, label it a reference state machine. It demonstrates accounting, rounding and rejection rules; it does not show that a Zcash transaction is parsed, a proof is sound, a signature is checked, or collateral is locked.

## Oracle and economic boundary

Zcash nodes can enforce protocol rules. A USD target still needs external information: Zcash cannot infer the dollar price of ZEC from its own ledger. Oracle providers may observe markets elsewhere and submit signed observations; nodes verify the allowed keys, quorum, epoch, bounds, freshness and deterministic aggregation entirely on chain.

Use integer/rational arithmetic with specified rounding. Require fresh prices for debt increases and collateral withdrawals. Define deterministic stale-price handling without unilateral operator overrides; repayments and safe collateral top-ups should remain possible when the oracle stalls. Rotation of oracle keys and risk parameters must be explicitly governed and delayed, or fixed by upgrade. A local prototype's quorum counter is not signature verification.

A ZEC-backed token targets a dollar value; it is not a claim on redeemable bank dollars. Collateral ratios, liquidation incentives, oracle delay, sudden ZEC price gaps, liquidity, redemption rules and bad-debt settlement determine peg resilience. Repayment alone does not guarantee a peg. Specify these economics before a production claim; the consensus engine must never invent collateral to conceal insolvency.

## Reserve model and proposed native integration

The signed Rust state machine now integrates the [reserve lifecycle](reserve-lifecycle.md) with vault balances, liabilities and the contextual seam above. Initial reserve ZEC comes from a named fixture donor; stable inventory starts at zero. Signed trades move tokens issued by model vaults. Immutable regtest funding, shutdown and auctions execute in the same private block overlay and share its conservation checks. [The integration specification](reserve-rust-contract.md) records the API and ordering. A native experiment still needs these consensus rules:

1. Recognize exactly one reserve value path holding native ZEC and already-issued zUSD. Reserve swaps transfer both assets atomically. They cannot change issued supply, debt or bad debt, release vault collateral, or authorize arbitrary assets. No discretionary spending or issuer key overrides these rules.
2. Fix bid/ask derivation, integer rounding, activation, expiry, inventory bounds, epoch gross outflow limits and the retained-funding cap through the network's canonical parameters. The model binds these parameters into its chain domain; native consensus must bind them through its own activation and parameter rules. All checked wide intermediates and stored-value bounds require specified rejection behavior.
3. Resolve the accepted ZEC/USD report from the exact parent branch and in-block ordering. Require freshness and canonical sequence/content. A user cannot select a convenient old price. Standing quotes require no stablecoin-market trigger feed. The Rust reserve uses report inclusion height plus signed expiry; its format has no signed market-observation timestamp, so a quorum can relabel old data as new. Native freshness policy and source trust need separate review.
4. Bind swap direction, quantity, actor authorization, both value flows, minimum output/maximum input, report sequence, derived epoch and expiry to the native transaction digest and chain/branch domain. Require real asset inputs and spend proofs or signatures. The Rust model verifies those intentions with Ed25519; native note and outpoint ownership still need Zcash authorization.
5. Carry reserve balances, derived epoch, per-epoch and lifetime gross counters, funding/NSM accounting and shutdown state in Zebra branch overlays, rollback data and finalized storage. Transactions in one block share cumulative limits. Lifetime gross counters are accounting records; the lifetime limit applies to retained funding, and sale proceeds do not restore that allowance.
6. Derive a separately approved funding allocation from the applicable coinbase subsidy schedule exactly once per block. Keep total coinbase/funding conservation intact. Retain credits only below both the funding target and lifetime budget; account for the remainder through the specified NSM rule. Market sale proceeds remain reserve working capital. The Rust schedule creates simulated ZEC and defaults to zero per block. It implements no native coinbase validation or NSM subsidy release.
7. Prepare swaps inside parent-specific contextual execution before native block commit. Roll back price references, balances and every counter together. Mempool allowance is provisional and cannot reserve a permanent quota. Revalidate on tip, oracle, epoch and activation changes.
8. Apply deterministic closure before a report could rescue an elapsed outage deadline or sunset. Burn actual reserve inventory against existing bad debt once, reduce supply by the same amount, route cash to the NSM path and sell residual tokens through the specified descending-price auction. Later liquidation losses do not retroactively change that burn. After closure all scheduled funding and auction proceeds go to NSM. The Rust ledger implements these model transitions; their native value paths and economic policy remain subject to review.

The relevant insertion points remain the dependency-level transaction/proof layer, common semantic verifier, validate_and_commit_non_finalized, commit_block, branch push/fork/pop_root, finalized atomic batches and mempool revalidation identified above. These are required changes, not implemented upstream APIs.

Additional native evidence must cover same-block quota races, reorgs spanning epoch boundaries and funding credits, duplicate funding attempts, oracle update ordering, stale signed quotes, inventory conservation, fee accounting, terminal policy and restart equivalence. The accounting experiments do not establish native consensus or cryptographic safety.

## Acceptance evidence still needed

The signed ledger closes the Python experiments' caller-authorization and funding-cadence gaps using actual model signatures, an immutable schedule and combined conservation. It remains an in-memory reference implementation. Returning cash to NSM even when inventory cannot cover all bad debt is an explicit economic policy, not a consequence required by accounting. The [combined stress study](combined-stress.md) exercises the signed ledger, the first Zebra-linked runner supplies block-binding and replay evidence, and the collateral profile checks native transparent ownership and per-transaction UTXO accounting. Native stablecoin value-pool enforcement, full node validation and durable branch-state evidence remain outstanding.

A complete native implementation needs, at minimum:

1. Frozen transaction, signing and proof statements, fixed asset constants, integer rounding, oracle policy, risk parameters and insolvency/settlement rules.
2. Reproducible dependency pins and cross-implementation serialization, txid, auth-digest and signature vectors, including invalid encodings and cross-branch replays.
3. Circuit tests proving unauthorized assets/minting/collateral release are impossible, plus independent cryptographic review.
4. Block tests for ordinary spends of vault collateral, duplicate vault use, same-block oracle ordering, aggregate issuance caps, burns, fee conservation and failure atomicity.
5. Fork/reorg, finalization, restart and migration tests showing the exact same state when replayed.
6. Real patched-node rejection/acceptance on an isolated network, with measured resource limits and a wallet flow; broader node review and upgrade approval before deployment.

Zebra's own test workflow uses `cargo nextest` (its AGENTS.md explains why plain `cargo test` is unsuitable for the full upstream suite). No full-node build, patched consensus execution, ZK circuit, network activation or production stablecoin is claimed by these research documents.
