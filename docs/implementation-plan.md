# Zcash Stablecoin Vault Implementation Plan

The completed research milestones and remaining integration work are listed below. Offline profiles link to Zebra and check fixture collateral spends; integration into the node verifier remains proposed. [verification.md](verification.md) records the tested scope.

Goal: build a single-stablecoin Zcash protocol proposal, executable Rust model and independent adversarial review.

The account model tests fixed asset types and vault rules off-network. The Zebra proposal identifies the transaction-library and branch-state changes needed in a native implementation against a pinned source tree.

Tools: Rust 2021, Ed25519 model signatures, restricted secp256k1 native signatures, integer arithmetic and Cargo tests; Markdown and PDF documents.

Specification: [design.md](design.md).

## Global Constraints

- ZEC and one protocol-defined stablecoin only; no generic asset registration or issuer keys.
- Regtest model only; public balances/vaults in v0; no real funds or production keys.
- Units: zatoshi and micro-zUSD; 250% borrow/withdraw, 200% liquidation, 5% bonus, minimum debt 100 zUSD, ceiling 100000 zUSD.
- Zero fees/interest in v0; canonical 2-of-3 oracle; no borrowing with bad debt.
- Preserve stable supply=live principal+bad debt, ZEC conservation and rollback on failure.

## Review Focus

- Replayed or cross-domain requests must not mutate state.
- Stale/future/duplicate-signer oracle reports must not authorize borrowing.
- Liquidation rounding, exact thresholds and zero-affordability dust must be deterministic.
- Unknown assets, overflow and aggregate supply limits must fail closed.
- Failed multi-action blocks must not leave partial balances, debt or oracle updates.

## Task 1: authenticated model and conservation

Files: `Cargo.toml`, `src/`, `tests/`, `examples/`.

- [x] Write adversarial tests before implementation; record red/green evidence in `tests/TDD.md`.
- [x] Implement fixed assets, authenticated requests/oracle, vault transitions, atomic block application, invariants and bounds.
- [x] Run tests and deterministic scenario covering healthy borrowing, repayment, liquidation and crash/bad debt.

## Task 2: native integration and research documents

Files: `docs/design.md`, `docs/whitepaper.md`, `docs/forum-post.md`, `docs/zebra-integration.md`, `docs/sources.md`, `zebra/`.

- [x] Pin current primary Zebra/ZIP sources; map serialization, transaction verification and branch-state boundaries.
- [x] Specify which ZSA components the design may reuse and the release checks for each stage.
- [x] Produce PDF whitepaper and review claims against executable scope.

## Task 3: independent security review

Files: `docs/threatmodel.md`, `docs/economic-review.md`, `docs/adversarial-review.md`, regression tests.

- [x] Write a threat model for the executable and proposed native implementation.
- [x] Fresh reviewer attempts attacks on code and documented assumptions.
- [x] Fix verified correctness issues, add regressions, retain economic/production blockers.

## Task 4: verified publication package

Files: `README.md`, `docs/verification.md`, `SECURITY.md`, `LICENSE`.

- [x] Run Cargo tests/fmt/clippy, scenario, PDF generation and bounded secret/content screening.
- [x] Inspect all staged files; commit original research and source only.
- [x] Push to the requested account/repository when existing authenticated permissions allow; verify remote commit and visibility. Do not widen permissions or copy credentials to overcome a scope error.

## Reserve lifecycle milestone

- [x] Replace lifetime gross trading stops with a nonresettable retained-funding budget in a separate research wrapper; preserve gross epoch limits and sunset.
- [x] Specify and test stale-oracle pause, fixed outage deadline, bad-debt inventory burn and paid inventory wind-down.
- [x] Review and remove a free terminal-distribution farming path; fix stale-report updates extending the outage indefinitely.
- [x] Publish the [lifecycle rules](reserve-lifecycle.md) and [signed Rust integration contract](reserve-rust-contract.md).
- [x] Implement the reserve and lifecycle inside the signed Rust vault state machine, including versioned domains, atomic funding and branch rollback.
- [x] Start with donor-funded ZEC and zero reserve tokens; acquire inventory from signed vault issuance and trades. Preserve the original vault-only vectors.
- [x] Bind reserve orders to sequence, epoch, expiry and slippage; retain the market-age limitation of inclusion-based freshness.
- [x] Stress the combined vault/reserve ledger with signed issuance, liquidator funding constraints, delayed reports, closure ordering and auction participation. Report latent active-vault shortfalls separately from recorded bad debt.
- [x] Compile and independently review the isolated Zebra-linked block runner, including payload commitments, parent selection and journal replay.
- [ ] Implement native transaction/value-pool rules, collateral locks, proofs, full consensus validation and durable Zebra branch state before a value-bearing network experiment.

## Transparent collateral milestone

- [x] Expose detached snapshots after the block prelude and each signed transaction through the model's shared validator.
- [x] Parse restricted V4 transparent transactions and verify canonical, low-S P2PK signatures against actual branch prevouts with `SIGHASH_ALL`.
- [x] Bind model owners to distinct native keys and sign native intents over the profile domain, parent checkpoint, unsigned native transaction and model request.
- [x] Anchor fixture UTXOs in a trusted bootstrap block containing its funding transaction; bind complete bootstrap bytes and owner mappings into the profile domain.
- [x] Pair every native user transaction with a model request, authorize every vault input/output and reconcile wallet and vault ZEC after each transaction. Keep model state and UTXOs atomic across failures and branch selection.
- [x] Restrict this profile to vault-only state, zero fees and simulated funding; reject reserve-enabled genesis, new funding coinbases, shielded paths and unsupported formats.
- [x] Complete independent adversarial validation and record the exact tested scope in [collateral-review.md](collateral-review.md).
- [ ] Install the guard in Zebra's ordinary transparent-spend path and protect checkpoint/direct finalization. Add a native stablecoin value pool, full consensus verification and durable branch state before any value-bearing experiment.

## Execution note

Separate agents implemented the model, researched Zebra, and reviewed economics and security. The main agent checked the combined package and published it. Private assistant context is excluded from the repository.
