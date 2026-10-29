# Test-first implementation evidence

The integration tests were written against a compiling, deliberately incomplete
interface before implementing state transitions. No cryptographic or economic
implementation was present in that scaffold; block validation returned
`Invalid("not implemented")`.

On 2026-10-29 (UTC), Rust 1.90.0:

```text
cargo test
tests/adversarial.rs: 17 tests; 11 passed, 6 failed; exit 101
```

Failures: fixture network guard; owner authorization setup; stale-price setup;
oracle rollback setup; oracle sequence setup; nonce replay setup. The eleven
negative tests that passed under blanket rejection are not independent RED
evidence; successful prerequisite transitions are checked by the completed suite.

```text
cargo test --test economics
8 tests; 0 passed, 8 failed; exit 101
```

Seven expected failures reached the unimplemented transition; the eighth
demonstrated the missing duplicate-allocation guard. Assertions cover issuance,
ratio/cap limits, repayment/closing, solvent liquidation, crash bad debt and its
borrow freeze, the liquidation threshold, debt dust, and genesis validation.

The managed environment used direct Cargo/Rustc binaries under
`/tmp/zsv-rustup/toolchains/1.90.0-x86_64-unknown-linux-gnu/bin` and
`CARGO_HOME=/tmp/zsv-cargo`. The first dependency fetch required network access.
An attempted rustup-proxy launch failed inside the sandbox before tests; this
environment failure is not counted as RED evidence.

Review then identified missing liquidator intent constraints. After adding the
new action fields to the type and signing encoding, but before enforcing them:

```text
cargo test --test economics liquidation_enforces_signed_sequence_max_burn_and_min_collateral
1 failed; exit 101: liquidation exceeded signed intent bounds
```

After enforcing canonical oracle sequence, maximum stablecoin burn, and minimum
collateral received, the same targeted command passed (1 passed, exit 0).

Final verification on 2026-10-29 (Rust 1.90.0):

```text
cargo test --locked --offline --all-targets
66 integration tests passed, 0 failed; example compiled; exit 0
cargo test --locked --offline
66 integration tests passed, 0 failed; doc tests passed; exit 0
cargo clippy --locked --offline --all-targets -- -D warnings
exit 0, no warnings
cargo fmt --all -- --check
exit 0
cargo run --locked --example scenario > examples/scenario-output.json
exit 0; 33,134-byte signed JSON vector artifact
```

The 66 tests comprise adversarial (22), boundaries (5), economics (9), independent
reviewer attacks (19), checked-in vector replay (1), and the proposed contextual
adapter seam (10). The adapter is a compiled model shim, not a linked or deployed
Zebra consensus change. Final scenario values are zero debt/supply for solvent
liquidation and 209,523,810 microUSD of both bad debt and outstanding supply after
the crash. The vector replay test verifies real signatures and complete final
snapshots/checkpoint hashes.

## Owner namespaces and immutable chain domains

Before publication, independent review changed two retained-limit tests into
security expectations **on the old implementation**, then ran:

```text
cargo test --offline --locked --test reviewer_attacks
17 passed, 2 failed; exit 101
```

`owner_scoped_vault_names_prevent_third_party_squatting` failed because an
attacker's open/close reserved the victim's global ID. The separate
`chain_domain_rejects_transaction_and_oracle_replay_between_distinct_configurations`
failed with both `tx_rejected=false` and `oracle_rejected=false`. Its two
configurations shared the two signing reporters and changed the third key, so
quorum membership could not mask missing domain authentication.

The implementation then added full owner-scoped `VaultId` keys (without hash
truncation), explicit liquidation owner references, canonical validated
configuration/genesis domains, and state-scoped signing helpers. The immutable
domain is signed by both transaction and oracle envelopes; rewriting it to the
destination domain invalidates the signature. An additional test verifies that
equivalent sorted fixtures/configurations and same-genesis forks share a domain,
which remains constant as their mutable checkpoints change.

Final verification after these fixes:

```text
cargo test --locked --offline
69 integration tests passed, 0 failed; doc tests passed; exit 0
cargo clippy --locked --offline --all-targets -- -D warnings
exit 0, no warnings
cargo fmt --all -- --check
exit 0
cargo run --locked --offline --example scenario > examples/scenario-output.json
exit 0; regenerated signed JSON artifact, 47,639 bytes
```

The final suite comprises adversarial (22), boundaries (6), economics (9),
independent reviewer attacks (21), vector replay (1), and contextual adapter
tests (10). The later results and regenerated vectors supersede the earlier
66-test checkpoint above.
