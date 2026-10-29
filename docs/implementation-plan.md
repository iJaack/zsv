# Zcash Stablecoin Vault Implementation Plan

The prototype and documents below are complete. Native Zebra support remains proposed; [verification.md](verification.md) records the tested scope.

Goal: build a single-stablecoin Zcash protocol proposal, executable Rust model and independent adversarial review.

The account model tests fixed asset types and vault rules off-network. The Zebra proposal identifies the transaction-library and branch-state changes needed in a native implementation against a pinned source tree.

Tools: Rust 2021, Ed25519 signatures, integer arithmetic and Cargo tests; Markdown and PDF documents.

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

## Execution note

Separate agents implemented the model, researched Zebra, and reviewed economics and security. The main agent checked the combined package and published it. Private assistant context is excluded from the repository.
