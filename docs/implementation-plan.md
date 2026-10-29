# Zcash Stablecoin Vault Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build a reproducible single-stablecoin native-Zcash research package with an executable Rust reference model and an independent adversarial review.

**Architecture:** Fixed asset types and a native vault state machine; an authenticated account model exercises the rules off-network. Zebra integration is separately specified against a pinned upstream tree, requiring transaction-library and contextual-state work rather than claiming that the model is already a native node.

**Tech Stack:** Rust 2021, Ed25519 signatures, integer arithmetic, Cargo tests; Markdown/PDF research artifacts.

**Spec:** [design.md](design.md)

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

## Task1: authenticated model and conservation

Files: `Cargo.toml`, `src/`, `tests/`, `examples/`.

- [x] Write adversarial tests before implementation; record red/green evidence in `tests/TDD.md`.
- [x] Implement fixed assets, authenticated requests/oracle, vault transitions, atomic block application, invariants and bounds.
- [x] Run tests and deterministic scenario covering healthy borrowing, repayment, liquidation and crash/bad debt.

## Task2: native integration and research documents

Files: `docs/design.md`, `docs/whitepaper.md`, `docs/forum-post.md`, `docs/zebra-integration.md`, `docs/sources.md`, `zebra/`.

- [x] Pin current primary Zebra/ZIP sources; map serialization, transaction verification and branch-state boundaries.
- [x] Specify exact single-asset/ZSA-subset boundary and staged implementation acceptance gates.
- [x] Produce PDF whitepaper and review claims against executable scope.

## Task3: independent security review

Files: `docs/threatmodel.md`, `docs/economic-review.md`, `docs/adversarial-review.md`, regression tests.

- [x] Threat model model-runtime and proposed native architecture separately.
- [x] Fresh reviewer attempts attacks on code and documented assumptions.
- [x] Fix verified correctness issues, add regressions, retain economic/production blockers.

## Task4: verified publication package

Files: `README.md`, `docs/verification.md`, `SECURITY.md`, `LICENSE`.

- [x] Run Cargo tests/fmt/clippy, scenario, PDF generation and bounded secret/content screening.
- [x] Inspect all staged files; commit original research and source only.
- [x] Push to the requested account/repository when existing authenticated permissions allow; verify remote commit and visibility. Do not widen permissions or copy credentials to overcome a scope error.

## Execution note

The user explicitly requested implementation and parallel adversarial review. Root handles integration/publication, a Rust worker handles the isolated model, a primary-source researcher handles Zebra, and independent reviewers handle economics and threat analysis. Private assistant context is excluded from all package content.
