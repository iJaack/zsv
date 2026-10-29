# Independent adversarial model review

Reviewed on 29 October 2026 by a separate code-review agent, using the public Rust API and deterministic fictional fixtures. This review covers the account-ledger prototype, not a deployed Zcash protocol. It found and reproduced namespace and replay weaknesses, then verified targeted regressions after their remediation. Resource limits and substantial economic/integration blockers remain. The results do not certify production security, economic solvency or a dollar peg.

## Scope and evidence

The reviewer read `AGENTS.md`, `SECURITY.md`, `Cargo.toml`, `docs/design.md`, `src/lib.rs` and the model's adversarial, economic and boundary tests. The final threat model was cross-checked for consistent classification. The reviewer wrote only this document and `tests/reviewer_attacks.rs`; implementation fixes belonged to the implementation worker. No private workspace context, credentials, real funds, network endpoint or production key was used.

The Codex Security `security-scan` instructions were read as review guidance. This delegated, offline review did not start a registered Security scan or produce a sealed Security report. Its evidence consists of the source and executable tests identified here.

The initial reviewed library had SHA-256 `9977d3bdda9b603a916455da1b4c992723cae5c9e133f1ff9e214f18037ad0e4`. The library after namespace/domain remediation has SHA-256 `b2f26bfba8c2d0ddb54b855e37d52741944a061ad368372a012002e1fcf535be`; `Cargo.lock` has SHA-256 `e44aec7ac75310c939371401fd77a3ecf3b2241fa81b50354ad359a5c1f39812`. These identify local artifacts because the repository did not yet have an implementation commit at review time. This is a one-pass source review plus targeted remediation checks, not a second full review, exhaustive state-space exploration or a dependency audit.

## Confirmed findings and remediation

Severity here describes the effect within a shared model deployment. There is no exposed service or real value in the reviewed setup. Consequences could be more severe if these choices were copied into a value-bearing protocol.

### R1: permanent global vault-ID squatting — medium, remediated

**Original root cause.** `Action::Open` accepted a caller-chosen global `u64` ID. `Open` recorded it in `used_vault_ids`, while `Close` removed the active vault but never released that historical ID. IDs were not scoped to an owner or derived from an authenticated transaction outpoint.

**Original reproduction.** A funded attacker signed `Open { vault_id: 7, collateral: 1 }` followed by `Close { vault_id: 7 }`. The block succeeded, all collateral returned, and no debt was incurred. Another owner's correctly signed `Open { vault_id: 7, ... }` was then rejected permanently. The reviewer first reproduced that behavior, then changed the probe to require the victim's open to succeed and observed `Invalid("vault id already used")` before the implementation fix.

The original attack denied a chosen identifier; it did not obtain the victim's collateral or prevent that owner from choosing another unused ID. Repeating open/close cycles at fresh local IDs still grows historical state at zero net ZEC cost after remediation. The 1,024-action block limit bounds growth per block to at most 512 such two-action cycles, but not cumulative growth. That distinct resource limitation remains R3.

**Remediation and evidence.** Active and historical vault keys now use the full pair `VaultId { owner: Pubkey, local_id: u64 }`. Owner operations resolve local IDs under the authenticated signer, while permissionless liquidation explicitly signs the targeted owner's full key. `owner_scoped_vault_names_prevent_third_party_squatting` now passes: the other owner's open/close cannot reserve the victim's namespace. `liquidation_owner_is_signature_bound_when_local_vault_ids_coincide` also verifies simultaneous local-ID-7 vaults for two owners and rejects a changed liquidation owner without a new signature. Native integration still requires actual outpoint/note semantics; the namespace fix does not implement collateral locking or resource pricing.

### R2: signatures could replay across distinct regtest genesis configurations — medium, remediated

**Original root cause.** `SignedTx::signing_bytes` authenticated the encoding version, `Network`, signer, nonce and complete action list, without a chain/genesis identity or immutable configuration. Oracle messages had the same isolation gap. A state commitment included the configuration, but that commitment was not part of the signed message domain.

**Original reproduction.** Initialize two regtest states with the same funded owner/recipient keys but different configured oracle keys. Their initial commitments differ. Submit exactly the same signed nonce-zero ZEC transfer to both: both originally accepted it. The pre-fix regression was strengthened to keep the first two reporter keys identical and change only the third reporter. Both the transfer and a signed oracle update then replayed successfully, so valid quorum membership could not mask the domain failure. The secure regression reported `transaction_rejected=false, oracle_rejected=false` before the fix.

The original transfer required compatible owner balances and nonces on each instance; oracle replay required a shared valid quorum. This was distinct from a repeated nonce on one history and did not bypass the hard mainnet/testnet guards.

**Remediation and evidence.** Transaction and oracle envelopes now authenticate `chain_domain`, a SHA-256 domain over the versioned model-chain prefix, network, sorted reporter keys and sorted validated genesis allocations. Signing prefixes are `zsv/tx/v2` and `zsv/oracle/v2`. Validation checks the expected immutable domain, and signatures bind it. `chain_domain_rejects_transaction_and_oracle_replay_between_distinct_configurations` now rejects both replays. `rewriting_domain_to_destination_does_not_transplant_signatures` rejects attempts to replace the domain with the destination's valid value without resigning, for both transactions and reports. A destination-signed request succeeds in the same fixture.

Identical canonical genesis fixtures intentionally identify the same model chain, including its forks; the domain is not a unique random deployment label. The implementation worker additionally tests canonical allocation/reporter ordering, changed allocations and fork-stable domains. A native implementation still needs its actual chain/activation and transaction-digest binding. General transaction expiry remains absent; liquidation's intent bounds address one narrower delayed-execution risk.

### R3: semantic count limits do not bound decoding or total state — medium, conditional on exposure

**Root cause.** Public wire structs contain ordinary serde `Vec` fields. A caller can deserialize an arbitrarily large signature, report list, transaction list or action list before `validate_block` checks its size. The API itself receives an already allocated `Block`. There is no byte-bounded parser, transport server, aggregate persistent-state budget or benchmarked native validation cost.

**Evidence.** The independent tests construct one-mebibyte transaction and oracle signature vectors and 1,000-report updates, then verify rejection and unchanged state. This shows rejection of those constructed values; it does not show that receiving them is memory-bounded. Code inspection establishes the allocation boundary. Transaction signature lengths and action/transaction counts are checked before expensive transaction signing-message construction. Oracle updates permit at most three reports before report verification. These are useful downstream limits.

**Status and required direction.** Retained; no network listener exists to exploit remotely here. Historical owner-scoped IDs and empty accounts still accumulate, and every validation clones the full state. Before adding a peer or file decoder, bound encoded bytes and collection lengths before allocation, limit aggregate verification work, and introduce reviewed state pricing, bounds or reclamation while preserving replay protection. No memory/CPU saturation test or denial-of-service benchmark was performed.

## Finding addressed during development

The threat-model review identified a validly signed liquidation request whose authorization originally specified only a vault ID. A producer could delay it until a different oracle price or changed vault state, changing the required burn and received collateral while the owner's nonce remained current.

The implementation worker added authenticated `oracle_sequence`, `max_burn` and `min_collateral` fields and checks them before any balance change. The reviewer observed the intent-bound regression failing before enforcement, then inspected the fixed signing encoding and checks. The final suite includes sequence/burn/receipt rejection tests and a signed-field binding test. This mitigates the identified execution-intent gap. It does not establish oracle truth, chain-instance isolation, general expiry or guaranteed inclusion, and permissive wallet-selected bounds remain permissive.

## Economic outcomes that remain blockers

These outcomes follow the documented rules. They are not hidden arithmetic inflation or authorization bypasses.

* **Self-liquidation can return all collateral while unsupported units remain.** Start with a 10-ZEC vault, borrow 400 zUSD at $100/ZEC, retain the borrowed units, then accept $20/ZEC. The owner can liquidate its own vault, burn 190.476190 zUSD, recover the entire 10 ZEC through seized collateral plus owner surplus, and retain 209.523810 zUSD. That remainder is exactly the recorded bad debt. `self_liquidation_conserves_assets_but_can_leave_unbacked_owner_held_supply` verifies this result. The accounting identity survives; economic backing does not. No recapitalization or loss-allocation mechanism exists.
* **Any bad debt freezes borrowing permanently in this model.** Existing transfers and repayment of other vaults still work, and total balances remain equal to live principal plus bad debt. This is covered by `after_bad_debt_existing_debt_can_be_repaid_and_supply_still_equals_balances`. There is no bad-debt coverage action to restore borrowing.
* **A liquidator may lack the full affordable burn.** The unaffordable liquidation probe verifies whole-block rollback, including the proposed price and prior actions. It does not create liquidation liquidity or liveness. A zero-affordable-burn dust close is permitted and provides no collateral reward to the submitter.
* **A signature quorum authenticates a tuple, not market truth or observation time.** Two matching configured signers can authorize any positive price within integer bounds. The expiry is limited to 100 blocks after inclusion, but there is no signed observation/start height or wall-clock freshness proof. Honest-source independence, reporter coordination/equivocation handling, rotation, source manipulation and price-gap loss remain external design problems.

The economic review and threat model discuss these limits in more detail. No market, liquidity, censorship or reporter-independence claim is established by these tests.

## Attack probes and observed boundaries

`tests/reviewer_attacks.rs` now adds 21 public-API tests. The original namespace/replay behavior is preserved above; the corresponding executable tests now require the corrected behavior. Economic limitation tests still intentionally demonstrate accepted bad-debt outcomes, and their passing status must not be interpreted as solvency or remediation of those economic blockers.

| Probe | Observed result |
| --- | --- |
| Unknown asset names, generic mint/registration actions, extra asset parameter and attempted ledger-field injection | Rejected by wire types. Altering a returned snapshot cannot alter private `State`. |
| Unknown fields on signed transaction, action, report, oracle update, block and configuration | Rejected at the modeled input boundaries. Public snapshot/account structs are not a state-import API. |
| Signer substitution and signed-action reordering | Rejected atomically. Existing tests also cover amount, recipient and nonce mutation. |
| Two distinct requests with the same signer/nonce in one block | Whole block rejected; consecutive nonces succeed and advance twice. Existing tests cover replay across blocks and wrong networks. |
| Self-transfers of ZEC and ZUSD | Preserve balances, supply and vault state, while consuming the transaction nonce. |
| Empty/short/long/one-mebibyte signature vectors; absent/one/excess oracle reports | Rejected with state and commitment unchanged. Allocation has already occurred. |
| Tampered report signer, network, sequence, expiry or signature | Rejected. Duplicate and unregistered reporters are covered by existing tests. |
| Independently signed reports disagreeing on sequence, price or expiry | Rejected. Reversing the order of matching reports produces identical final state and commitment. |
| Expiry 101 blocks beyond inclusion versus exactly 100 | First rejected; boundary accepted. No start/observation height exists to test. |
| `u64::MAX` monetary action amounts | Fail closed without mutation. Separate existing tests check checked-ratio overflow and the exact supply cap. |
| Maximum `u64` total fixture value, self-transfer and full-balance transfer | Succeed without wrap or supply creation. Fixture total overflow is separately rejected. |
| Borrow below minimum; repay leaving nonzero dust | Rejected atomically; full repayment and closure succeed. |
| Unfunded liquidation after a valid earlier action and proposed oracle update | Whole block rejected; price, nonce, balances and prior actions roll back. |
| Insolvent self-liquidation and repayment of a separate vault after bad debt | Explicit shortfall retained; ZEC and stable accounting continue to hold. |
| Vault ID squatting and cross-genesis regtest replay | Originally reproduced; final namespace and signature-domain regressions pass after R1/R2 remediation. |
| Rewritten destination domain and changed liquidation owner | Rejected without resigning; identical local IDs can coexist under different owners. |
| Pre-funding another public key | Creates its account at nonce zero; does not obtain its signing authority or consume its first nonce. |

No tested public instruction created arbitrary assets, ZEC or stablecoin supply outside the documented borrow path; no tested malformed block partially committed. This is bounded evidence from the listed probes, not a claim that all possible inputs or traces are safe.

## Reproduction and exact verification status

Use the Rust 1.90 toolchain and downloaded dependencies from the workspace preparation:

```sh
cd /workspace/zsv
export CARGO_HOME=/tmp/zsv-cargo
export RUSTC=/tmp/zsv-rustup/toolchains/1.90.0-x86_64-unknown-linux-gnu/bin/rustc
/tmp/zsv-rustup/toolchains/1.90.0-x86_64-unknown-linux-gnu/bin/cargo test --offline --locked --test reviewer_attacks
```

The final independent targeted run completed at approximately 22:32 UTC on 2026-10-29: **21 passed, 0 failed, 0 ignored**. The reviewer directly formatted that test file with Rust 1.90 `rustfmt`. Its formatted SHA-256 is `b4b68bfeb05d7c37112421a45cbbf48de00b6addb1e70728c91ab7a459612110`.

An initial offline command without the prepared `CARGO_HOME` failed dependency resolution (`serde` was not found); setting the cache path resolved that environment issue. An intermediate full-suite run captured the deliberately failing liquidation-intent regression while enforcement was still being implemented. After initial review, the two R1/R2 tests were changed to require safe behavior against the unchanged original library: the targeted run returned exit 101 with **17 passed and 2 failed**, for the exact namespace and transfer/oracle replay assertions described above. After the source fix and API migration, those regressions plus the two additional signature-binding probes passed in the final 21-test run. These intermediate failures are not final green results. The implementation worker/root's final verification receipt records the complete test, formatting and Clippy checks after all files were present; this targeted receipt alone makes no whole-project validation claim.

## Deferred work and review limits

No Zebra parser, native ZEC lock, transaction digest, consensus activation, side-branch database, real reorg, shielded circuit, wallet, mempool, PoW or peer decoder is implemented here. The review cannot establish their security from the account model. Native integration needs separate adversarial tests for collateral unspentness, per-asset mint/burn proof binding, serialization and digest vectors, fork/restart equivalence, bounded decoding and validation, and persistent state costs. The source-pinned integration proposal is a plan, not evidence that those boundaries are enforced.

This review did not perform fuzzing, exhaustive arithmetic/property exploration, independent cryptographic analysis of Ed25519 or dependencies, dependency-advisory checking, a hostile-input performance campaign, or economic simulation. Oracle compromise/withholding, censorship, key loss, governance, privacy and a credible peg/settlement mechanism remain substantial unresolved risks. Re-review the final committed patch and any native implementation before making broader claims.
