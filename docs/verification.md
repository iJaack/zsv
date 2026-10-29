# Verification receipt

Status: local implementation and checks complete; published to the public iJaack/zsv repository on 29 October 2026 with explicit user authorization.

## Scope

The checks cover an account-ledger model and a contextual preparation/commit adapter compiled as integration-test code. Native Zcash transactions, a full Zebra node, consensus activation and real funds remain outside that scope. The forum post and upstream proposal are drafts.

The native integration proposal uses Zebra `5faf8a413f0d30a6e93115f3d16af1e229fd7858` and the ZIP pin recorded in `sources.md`. The researchers inspected those sources; they did not build a patched node.

## Reproduction

```sh
bash scripts/verify.sh
bash scripts/build-whitepaper.sh
```

The first command runs locked Cargo tests, doc tests, formatting, strict Clippy, scenario regeneration comparison, relative-link/credential-pattern checks and a whitespace check. The second uses Graphviz, Pandoc and pdfLaTeX to regenerate the diagram and paper. Rust 1.90 is selected by `rust-toolchain.toml`; Cargo may need a network connection for its initial dependency download.

## Observed final results

After the namespace and signature-domain fixes, the main agent ran the complete verification script. It exited with status zero.

| Check | Observed result |
|---|---|
| Adversarial integration tests | 22 passed |
| Boundary integration tests | 6 passed |
| Economics integration tests | 9 passed |
| Independent reviewer attack tests | 21 passed |
| Signed scenario vector replay | 1 passed |
| Contextual adapter tests | 10 passed |
| Total integration tests | 69 passed; 0 failed; 0 ignored |
| Unit / doc / example test targets | Successful; no additional tests defined |
| Rust formatting | Passed |
| Strict Clippy, all targets | Passed; warnings denied |
| Scenario regeneration | Byte-for-byte match with checked-in signed vectors |
| Package screening | 36 source/artifact files; relative links and bounded credential patterns passed |
| Git whitespace check | Passed |
| Whitepaper | 10-page PDF generated; cover and interior rendering inspected |

The scenario contains a solvent liquidation with supply/bad debt both zero, and a crash case leaving 209,523,810 micro-zUSD as explicit bad debt. Its final model checkpoints are:

- Solvent: `1efc6b48c5a1a5c32035fa3cd43fb8e561c27c3d912ee8ccb802659fe1ece759`.
- Crash: `03afd614880bf7efe186b4bcfb02f70dbf2588d75de5dc6f79f22d1de7a6d5ac`.

The model uses these fingerprints to check repeatable state transitions. A native implementation needs Zcash block hashes and consensus state commitments.

## Source fingerprints

These hashes identify the current files after prose and code cleanup. The adversarial review records the earlier source versions it examined.

- `src/lib.rs`: SHA-256 `d05503f59afe6324ee4a5119e41aba5bb845411a772f738f272db68edf6fdc07`.
- `Cargo.lock`: SHA-256 `e44aec7ac75310c939371401fd77a3ecf3b2241fa81b50354ad359a5c1f39812`.
- `tests/reviewer_attacks.rs`: SHA-256 `b4b68bfeb05d7c37112421a45cbbf48de00b6addb1e70728c91ab7a459612110`.
- `docs/whitepaper.pdf`: SHA-256 `f694eea785580dc3bb2134b8921befc917524231b0d23bd91564c5cfc2384a62`.

PDF generation timestamps may change on rebuilding; source behavior and the signed scenario vectors are deterministic.

## Test-first and independent review evidence

`tests/TDD.md` records the initial failures and subsequent passes. Blanket-rejection negative tests in the initial scaffold are not counted as independent failing-test evidence. The original 66-test suite preceded the final hardening and is not the final release receipt.

The independent reviewer reproduced global identifier squatting and transaction/oracle replay across different regtest configurations. The reviewer wrote assertions for the expected safe behavior and observed them fail before the source changes. Owner-scoped identifiers and canonical genesis/configuration-bound signing domains corrected those behaviors; the final independent targeted run passed 21 tests. Signed liquidation epoch/burn/receipt bounds were also added after a failing reproduction. These targeted checks do not constitute a second complete security audit.

The independent arithmetic review sampled 239,600 deterministic integer cases using Python's unbounded integers. Those samples check the formula inequalities. Rust boundary tests check bounded arithmetic and overflow. Neither establishes market execution or economic solvency.

## Environment and limits

This managed environment installed an isolated Rust 1.90 toolchain under `/tmp/zsv-rustup`, with Cargo's dependency cache under `/tmp/zsv-cargo`. The rustup shim failed under the execution sandbox, so the main agent used the direct compiler/Cargo binaries with that cache and offline dependency resolution. This environment-specific workaround does not change the repository's portable commands.

`scripts/check_package.py` checks selected credential patterns and relative links. It does not replace a full secret scan or dependency security audit. Fixture keys are intentionally public and fictional. No private workspace identity, credentials or unrelated source is part of the package.

Before deployment, ZSV needs byte-bounded decoding, limits or fees for state growth, native parsing and proofs, collateral locking, node fork and persistence tests, oracle governance, peg mechanisms, recapitalization and shielded payments. The protocol cannot prevent off-protocol derivatives or speculation. No production safety, privacy or dollar-peg guarantee follows from these tests.

## GitHub delivery

The authenticated account is `iJaack`. Initial repository creation was blocked by the integration's permissions; the user then created `iJaack/zsv` and explicitly authorized keeping it public and pushing this reviewed package.

The remote already contained its initial README commit `8509bb5`. That commit was merged without force-pushing or deleting its history. The expanded ZSV README preserves the original project name and description.

The public push succeeded. An independent GitHub REST read of `repos/iJaack/zsv/commits/main` returned `c102a18d03e928954ac9634166b19e3ca11d339d`, the merge containing the full reviewed package. A later documentation commit added this receipt; the repository history records that commit.

Repository: <https://github.com/iJaack/zsv>. No forum post, upstream PR, deployment or network activation followed from this source publication. No credential or permission expansion was used.

## Whitepaper attribution update

The whitepaper credits Giacomo Barbieri. Its Markdown author field, PDF title page and PDF metadata were checked after regeneration. That attribution update changed no executable code.

## Prose and code cleanup

The prose pass applied [No AI Slop](https://github.com/petergyang/no-ai-slop) to the README, repository instructions, security policy, research documents, forum draft and test evidence. It removed filler, repeated caveats and decorative emphasis. Equations, parameters, requirements, source links, findings and Giacomo Barbieri's authorship were preserved. The rebuilt PDF has 10 pages; its title page and metadata credit Giacomo Barbieri.

The code pass followed [AI Slop Cleaner](https://buildwithclaude.com/skill/ai-slop-cleaner). It consolidated adjacent numeric signing-encoding calls in `src/lib.rs` using the existing private helper. `tests/cleanup.rs` adds two literal-byte tests covering all eight action variants, both assets, transaction and oracle headers, and integer boundaries. The fixtures passed against the old encoder before the refactor and against the new encoder afterward.

The writer, a separate reviewer and the main agent each ran `scripts/verify.sh` successfully after the code edit: 71 integration tests passed, including the original 69; doc tests, formatting, strict Clippy, exact scenario comparison, package checks and whitespace checks passed. The reviewer found no required follow-ups. The package checker now covers 38 files.

Cargo manifests, dependencies, public APIs, signature bytes, checkpoints, errors and monetary rules are unchanged. The [cleanup plan and evidence](code-cleanup.md) record the diff and remaining risks. The Dev Fund incentive discussion remains a funding proposal; this cleanup adds no rewards or monetary mechanism.
