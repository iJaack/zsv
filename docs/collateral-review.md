# Independent collateral-profile review

Reviewed 30 October 2026 against the working tree based on `9ffaf20ec6e0bdd7d8b5ce2bf72d0b9151970869` in `/workspace/zsv`. This is an independent coding-agent review of an isolated prototype, not an external audit or a production consensus assessment. The reviewer owned only `zebra/dev-harness/tests/collateral_review.rs` and this receipt; the core and wire implementations were written by separate agents. No commits, pushes, node services or network activations were performed by this reviewer.

## Findings and disposition

**Native expiry was signed but not enforced against the candidate height. Fixed.** The reviewer independently constructed a valid signed model transfer, a genuine native P2PK signature and an Ed25519 native intent with transaction expiry 1. The first child of the custom bootstrap has native height 2. Before the fix the harness accepted it. The red execution is recorded in `/tmp/zsv-collateral-review-red.log`; this was an acceptance failure, not a compile failure. Core now requires a parsed expiry at least equal to the candidate native height. The regression also accepts expiry 2 at height 2. This was a validity-condition defect inside the experiment; no theft or live-chain effect was demonstrated.

**Aggregate fixture issuance could exceed the native monetary range. Fixed by the core implementer.** This reviewer agreed with the source finding and read the implementation agent's red execution in `/tmp/zsv-collateral-money-red.log`: two individually valid MAX_MONEY allocations were accepted together. The implementer added the regression and the constructor's checked `Amount<NonNegative>` conversion of `fixture_zec_total`. Conservation then bounds later native totals. The implementation agent reproduced this issue; it was not independently reproduced by this reviewer. The initial combined green log included successful core tests and an unrelated, subsequently corrected test-builder failure in this reviewer's then-in-progress suite.

**A zero-expiry helper path could reach an `expect` on an absent expiry. Fixed separately.** The core/wire implementers identified and addressed this while integrating. Wire shape validation now requires a finite positive expiry below 500,000,000 before those accesses. This review inspected the fix and the wire tests, but did not independently reproduce that panic before the fix.

No additional authorization or backing defect was found in the bounded cases below. This statement does not establish absence of other defects.

## Independent execution

`cargo nextest run --locked --manifest-path zebra/dev-harness/Cargo.toml --test collateral_review --no-fail-fast` passed **13 tests**, with zero skips. The final output is `/tmp/zsv-collateral-review-green.log`. `cargo clippy --locked --manifest-path zebra/dev-harness/Cargo.toml --test collateral_review -- -D warnings` passed; output is `/tmp/zsv-collateral-review-clippy.log`. The test file was formatted with Rust 1.90 rustfmt. Runs used the existing offline dependency cache, Rust 1.90, the shared isolated target directory and the separate harness lockfile.

The tests cover:

- Native expiry at the exact candidate-height boundary, with atomic rejection.
- P2PK signatures against the actual resolved amount and all outputs; rejection of SIGHASH_NONE, SINGLE, ANYONECANPAY and other flags, a nonminimal push, trailing script data and a mathematically equivalent high-S signature. Mutated transaction bytes are decoded again by Zebra.
- A nonexistent outpoint attached to a signed vault opening, and a different owner's valid native signature paired with a valid model-owner intent.
- Conservation without correct owner balances, direct output inflation, repeated inputs and reuse by a later transaction in the same block.
- A two-transaction block whose final ZEC totals match but whose intermediate backing does not. The entire block is rejected unchanged.
- Zero-delta rotation of another owner's vault under an unrelated valid action, and a native transaction omitted from the model-operation list.
- Withdrawal and closure outputs redirected to another owner or duplicated, followed by a valid close.
- Liquidation that withholds the owner's surplus, followed by the exact valid liquidator/owner payouts.
- Branch-local prevouts, mismatched parent checkpoints, valid extension of a nonselected parent, and preservation of sibling/genesis states.
- Different native bootstrap transaction identity despite identical model balances; different profile domain/checkpoint; a cross-profile intent; duplicate native keys in owner bindings.
- Mainnet and public Testnet rejection, plus rejection of reserve-enabled genesis even with zero reserve funds and zero synthetic funding rate.

Native rejection helpers compare the whole archive before and after rejection. Single-operation attack envelopes explicitly validate their model block first, so rejection cannot be credited to an unrelated invalid model request. This preflight caught a withdrawal fixture below the model's 250% withdrawal ratio; the fixture now repays first and exercises native payout enforcement. Multi-operation backing and double-spend attacks likewise validate the model block first.

## What the design establishes, and what remains

The strongest parts of this slice are authoritative branch UTXO lookup, separate native and model signatures, exact per-transaction bucket equality, explicit vault-action authorization even when values do not change, complete native transaction pairing, and candidate-only mutation until the whole block succeeds. Bootstrap domain separation binds actual serialized funding-block identity and the sorted model/native key mapping. State checkpoints include native outpoints and coin metadata as well as the model commitment.

Existing model transaction and oracle signatures retain the model-genesis domain. The new native intent adds the actual collateral-profile domain, the exact parent checkpoint, the unsigned native transaction and canonical signed model request. This is additional binding, not a change to legacy model or oracle signature semantics.

The profile is still an offline experiment. Its vault script ends in `OP_TRUE` after dropping a fixed marker and is spendable without the contextual guard on an unmodified node. No production Zebra hook is installed. A node integration must cover all transparent inputs, ordinary unannotated spends, non-finalized and checkpoint/direct-finalization paths, and matching persistence/rollback and mempool behavior. Calling the standalone guard deployed consensus would be incorrect.

Other concrete limits remain: a custom trusted unmined funding bootstrap with a maturity exception; no ordinary PoW/subsidy/full-script validation; fixed Sapling signature context rather than upgrade-aware consensus; zero fees; immutable pre-registered owner mappings; stablecoin balances still held in the model; no shielded collateral or native stablecoin pool; reserves and synthetic funding rejected; and no authenticated journal for this new profile. The legacy carrier journal does not provide persistence for this profile.

The finite UTXO/block/count/byte caps bound this harness but are not production denial-of-service defenses. In particular, zero-value wallet outputs and zero-fee transactions permit cheap UTXO expansion within the cap. No production CPU/memory benchmark, randomized fuzz campaign, formal proof, or full-node bypass audit was performed here. Oracle correctness, liquidation-market capacity and economic stability are not established by these collateral tests.

Source SHA-256 at completion:

```text
0053a1996b70c965c9c6f3f84a45f8625c41016b7be8982666eda974c1c20c55  src/lib.rs
1f6f3db24d1109bad4b53c997ec637a45684ac4b94edf60041ff9346c89ed9a9  zebra/dev-harness/src/collateral.rs
edcdffb723fd1822c1428de497c04e0298c666094050f7399a7fd482c557d2dc  zebra/dev-harness/src/collateral_wire.rs
0cf5168bfa5f56726e6998c84d019ed8b086695f6979e44748712257edb80a0a  zebra/dev-harness/tests/collateral_review.rs
```
