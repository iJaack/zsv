# Verification receipt

Status: local implementation and checks complete; published to the public iJaack/zsv repository on 29 October 2026 with explicit user authorization.

## Scope

The executable under test is an isolated account-ledger model. A contextual preparation/commit seam is compiled as integration-test code. Neither is an activated Zcash transaction format or a full Zebra node. No real funds, production keys, consensus activation, public node, forum publication or upstream PR was used.

The native integration proposal is researched against Zebra `5faf8a413f0d30a6e93115f3d16af1e229fd7858` and the ZIP pin recorded in `sources.md`. Those sources were inspected, not built as a patched node.

## Reproduction

```sh
bash scripts/verify.sh
bash scripts/build-whitepaper.sh
```

The first command runs locked Cargo tests, doc tests, formatting, strict Clippy, scenario regeneration comparison, relative-link/credential-pattern checks and a whitespace check. The second uses Graphviz, Pandoc and pdfLaTeX to regenerate the diagram and paper. Rust 1.90 is selected by `rust-toolchain.toml`; Cargo may need a network connection for its initial dependency download.

## Observed final results

The main agent independently ran the complete verification script after the implementation worker and independent reviewer finished the namespace/domain fixes. Exit status was zero.

| Check | Observed result |
|---|---|
| Adversarial integration tests | 22 passed |
| Boundary integration tests | 6 passed |
| Economics integration tests | 9 passed |
| Independent reviewer attack tests | 21 passed |
| Signed scenario vector replay | 1 passed |
| Contextual adapter tests | 10 passed |
| Total integration tests | **69 passed; 0 failed; 0 ignored** |
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

These are model fingerprints, not Zcash block hashes or consensus state roots.

## Source fingerprints

- `src/lib.rs`: SHA-256 `b2f26bfba8c2d0ddb54b855e37d52741944a061ad368372a012002e1fcf535be`.
- `Cargo.lock`: SHA-256 `e44aec7ac75310c939371401fd77a3ecf3b2241fa81b50354ad359a5c1f39812`.
- `tests/reviewer_attacks.rs`: SHA-256 `b4b68bfeb05d7c37112421a45cbbf48de00b6addb1e70728c91ab7a459612110`.
- `docs/whitepaper.pdf`: SHA-256 `e97dc1be48d40ab3bfb492d4bb52af98a08968c7a8c4b11af05a7ef5a3ca8871`.

PDF generation timestamps may change on rebuilding; source behavior and the signed scenario vectors are deterministic.

## Test-first and independent review evidence

Initial failures and subsequent passes are recorded in `tests/TDD.md`. Blanket-rejection negative tests in the initial scaffold are not counted as independent failing-test evidence. The original 66-test suite preceded the final hardening and is not the final release receipt.

The independent reviewer reproduced global identifier squatting and transaction/oracle replay across different regtest configurations. Secure regression assertions failed before the source changes. Owner-scoped identifiers and canonical genesis/configuration-bound signing domains corrected those behaviors; the final independent targeted run passed 21 tests. Signed liquidation epoch/burn/receipt bounds were also added after a failing reproduction. These targeted checks do not constitute a second complete security audit.

The independent arithmetic review sampled 239,600 deterministic integer cases using Python's unbounded integers. This checks formula inequalities only, not bounded Rust overflow behavior, market execution or economic solvency. Rust boundary tests exercise checked overflow separately.

## Environment and limits

This managed environment installed an isolated Rust 1.90 toolchain under `/tmp/zsv-rustup`, with Cargo's dependency cache under `/tmp/zsv-cargo`. The rustup shim failed under the execution sandbox, so the main agent used the direct compiler/Cargo binaries with that cache and offline dependency resolution. This environment-specific workaround does not change the repository's portable commands.

`scripts/check_package.py` performs bounded pattern and relative-link checks, not comprehensive secret detection or a dependency security audit. Fixture keys are intentionally public and fictional. No private workspace identity, credentials or unrelated source is part of the package.

Retained blockers include pre-allocation decoder/resource limits, cumulative state growth/pricing, native parsing/proofs/collateral locking, full-node fork/persistence behavior, oracle governance, a credible peg, recapitalization and shielded payments. The protocol cannot prevent off-protocol derivatives or speculation. No production safety, privacy or dollar-peg guarantee follows from these tests.

## GitHub delivery

The authenticated account is `iJaack`. Initial repository creation was blocked by the integration's permissions; the user then created `iJaack/zsv` and explicitly authorized keeping it public and pushing this reviewed package.

The remote already contained its initial README commit `8509bb5`. That commit was merged without force-pushing or deleting its history. The expanded ZSV README preserves the original project name and description.

The public push succeeded. An independent GitHub REST read of `repos/iJaack/zsv/commits/main` returned `c102a18d03e928954ac9634166b19e3ca11d339d`, the merge containing the full reviewed package. This receipt is added in a subsequent documentation commit; the repository history records its final identifier.

Repository: <https://github.com/iJaack/zsv>. No forum post, upstream PR, deployment or network activation followed from this source publication. No credential or permission expansion was used.
