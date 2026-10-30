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

The first command runs locked Cargo tests, doc tests, formatting, strict Clippy, signed scenario comparison, Python mathematical tests, numeric study reproduction, relative-link/credential-pattern checks and a whitespace check. The second uses Graphviz, Pandoc and pdfLaTeX to regenerate the diagram and paper. Rust 1.90 is selected by `rust-toolchain.toml`; Cargo may need a network connection for its initial dependency download. The mathematical study needs Python 3.10 or later; Matplotlib is optional for figure regeneration.

## Observed final results

After integrating the reserve into the signed Rust ledger and applying the independent review fix, the main agent ran the complete verification script. It exited with status zero. The final documentation/package check also passed after the review receipt was added.

| Check | Observed result |
|---|---|
| Adversarial integration tests | 22 passed |
| Boundary integration tests | 6 passed |
| Signing cleanup integration tests | 2 passed |
| Economics integration tests | 9 passed |
| Independent reviewer attack tests | 21 passed |
| Signed scenario vector replay | 1 passed |
| Contextual adapter tests | 10 passed |
| Reserve core tests | 8 passed |
| Independent reserve attacks | 19 passed |
| Combined reserve adapter tests | 3 passed |
| Signed reserve vector replay | 1 passed, covering three scenarios |
| Total Rust integration tests | 102 passed; 0 failed; 0 ignored |
| Python mathematical tests | 96 passed: 71 previous, 22 lifecycle, 3 independent lifecycle review tests |
| Numerical study reproduction | Exact JSON/CSV match; funding 10 scenarios / 240 combinations; reserve 11 / 144; lifecycle accounting cases |
| Unit / doc / example test targets | Successful; no additional tests defined |
| Rust formatting | Passed |
| Strict Clippy, all targets | Passed; warnings denied |
| Scenario regeneration | Both examples match checked-in JSON exactly; legacy vectors unchanged |
| Package screening | 76 source/artifact files; relative links and bounded credential patterns passed |
| Git whitespace check | Passed |
| Whitepaper | 12-page PDF generated; Giacomo Barbieri author metadata and reserve/lifecycle pages checked |

The scenario contains a solvent liquidation with supply/bad debt both zero, and a crash case leaving 209,523,810 micro-zUSD as explicit bad debt. Its final model checkpoints are:

- Solvent: `1efc6b48c5a1a5c32035fa3cd43fb8e561c27c3d912ee8ccb802659fe1ece759`.
- Crash: `03afd614880bf7efe186b4bcfb02f70dbf2588d75de5dc6f79f22d1de7a6d5ac`.

The model uses these fingerprints to check repeatable state transitions. A native implementation needs Zcash block hashes and consensus state commitments.

## Source fingerprints

These hashes identify the current files after the signed reserve integration. The adversarial review records the earlier source versions it examined.

- `src/lib.rs`: SHA-256 `efa3702d818a0453863b6e59bc7a8d11393c1802c9c5ae86541a10c303a9ffa6`.
- `src/reserve.rs`: SHA-256 `a219be80b71985a984b84eb626057908a001b538914d9bb9893fe09bd6c07c81`.
- `tests/reserve_attacks.rs`: SHA-256 `7a16cff322c82f34fecdce60acfc64a425ed4152ebe68382114a6d6006ca9208`.
- `examples/reserve-scenario-output.json`: SHA-256 `575a1cea502679d94083240d9ebad7395c647182c729a1ba90d1de2dde649895`.
- `Cargo.lock`: SHA-256 `e44aec7ac75310c939371401fd77a3ecf3b2241fa81b50354ad359a5c1f39812`.
- `tests/reviewer_attacks.rs`: SHA-256 `b4b68bfeb05d7c37112421a45cbbf48de00b6addb1e70728c91ab7a459612110`.
- `docs/whitepaper.pdf`: SHA-256 `aeb112784c64858d3a5fa26889d2dba6561f1d54c65f887801516e4e32643d04`.
- `analysis/stabilization.py`: SHA-256 `b7a1524f690901ab104305c7c95ac503034ae08e4fd2a99823ddabfaa92dfc75`.
- `analysis/run_study.py`: SHA-256 `128a5567f539244ff079c4991daff98929223b8b3912a34f50ba24f197bf114e`.
- `analysis/results.json`: SHA-256 `5f846c22ab50554d0b15d2ef68767b30702cabf4b775166afea7e0c0e853d845`.
- `analysis/reserve.py`: SHA-256 `41ef7fd01d5d629a291d9ba34cbc05d2b57a30cf70a3b2d32eba788029bf9b51`.
- `analysis/reserve_study.py`: SHA-256 `45c711987565e2aa6ccf22717e4d5a3a2845d111980edef28a1df2f4c50accb9`.
- `analysis/reserve-results.json`: SHA-256 `f9c053e69227e524e7c33f1e70adf1829d7828f755ddcb0f3eaa516ee89b3777`.
- `analysis/reserve_lifecycle.py`: SHA-256 `767e175360c3a0ae1c5b3d066e2f4002b418da96561ec18a104895e5003d37aa`.
- `analysis/lifecycle_study.py`: SHA-256 `2eda95380fc12fa6da6d9168acfc65a83bf67b1be52d94edfa79145a4305e1f3`.
- `analysis/lifecycle-results.json`: SHA-256 `1d0f26e62d8a517937d2ba2dc49f1299cb0a604d2c758dd564707d942159a28e`.

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

The prose pass applied [No AI Slop](https://github.com/petergyang/no-ai-slop) to the README, repository instructions, security policy, research documents, forum draft and test evidence. It removed filler, repeated caveats and decorative emphasis. Equations, parameters, requirements, source links, findings and Giacomo Barbieri's authorship were preserved. At that cleanup milestone the rebuilt PDF had 10 pages; its title page and metadata credit Giacomo Barbieri.

The code pass followed [AI Slop Cleaner](https://buildwithclaude.com/skill/ai-slop-cleaner). It consolidated adjacent numeric signing-encoding calls in `src/lib.rs` using the existing private helper. `tests/cleanup.rs` adds two literal-byte tests covering all eight action variants, both assets, transaction and oracle headers, and integer boundaries. The fixtures passed against the old encoder before the refactor and against the new encoder afterward.

The writer, a separate reviewer and the main agent each ran `scripts/verify.sh` successfully after the code edit: 71 integration tests passed, including the original 69; doc tests, formatting, strict Clippy, exact scenario comparison, package checks and whitespace checks passed. The reviewer found no required follow-ups. The package checker now covers 38 files.

Cargo manifests, dependencies, public APIs, signature bytes, checkpoints, errors and monetary rules are unchanged. The [cleanup plan and evidence](code-cleanup.md) record the diff and remaining risks. The Dev Fund incentive discussion remains a funding proposal; this cleanup adds no rewards or monetary mechanism.

## Mathematical stabilization study

The [report](stabilization-math.md) and [executable study](../analysis/README.md) test a hypothetical diversion of future grant-funding reductions. Funding splits use `Fraction` and integer zatoshi. Market trajectories use floating-point equations with explicit, uncalibrated response assumptions. They are separate from consensus monetary arithmetic.

The main agent verified 32 Python tests, 10 finite-budget scenarios, 240 parameter combinations, and byte-for-byte reproduction of JSON/CSV results. Both figures were regenerated with Matplotlib 3.10.8 and visually inspected. Numeric checks use only Python's standard library. The package checker excludes ignored Python bytecode caches.

An independent mathematical reviewer checked the funding conversions, analytical stability bounds, reserve reconciliation and attack examples, then reran all 32 final Python tests and numeric reproduction successfully. Twelve additional unconstrained cases matched the analytical recurrences to less than 1e-14. A separate source reviewer independently recomputed the signed funding comparisons and checked the grant-expiry baseline. Review caught malformed math delimiters and funding-denomination ambiguities; these were corrected before publication.

The grid's endpoint proximity statistic is not a convergence or success probability: some bounded runs stop at the target through exact reserve exhaustion despite an unstable ideal gain. No empirical calibration, production reward implementation, stablecoin price oracle, liquidation simulator or dollar-redemption facility was added. Rust source, dependencies and signed vectors retain their previous hashes. The rebuilt whitepaper continues to credit Giacomo Barbieri.

## Inventory reserve implementation and review

The [reserve report](reserve-study.md) covers a new exact-integer Python state machine and a separate assumed market. It conserves each asset across atomic bid/ask fills, holds purchased token inventory, preserves aggregate stablecoin liabilities and enforces physical gross limits. Funding amounts/cadence, canonical policy and observations, branch state and actor authorization are trusted host inputs. No native transaction or reserve operation was added to Rust or the compiled Zebra adapter.

All 71 Python tests pass, including 39 new tests. Numeric reproduction matches 11 reserve scenarios and 144 parameter combinations exactly. Independent reviewers reran the suite and reproduction; one also checked 1,000 randomized fills against Fraction-derived rounding inequalities, balance deltas, nonces and conservation. The separate adversarial suite exercises 72 unchanged-oracle cycles and verifies hostile-price value transfers and quota occupation. These samples do not establish an adversarial equilibrium or calibrated price response.

Review reproduced a working-capital drain when sale proceeds above a hard cash cap swept into the NSM bucket. The implementation agent observed the 101-ZEC regression expectation fail against that behavior, then fixed the policy to retain market proceeds. The new-funding target diverts excess subsidy credits only. Independent replay now finishes the 20-epoch path with 101 ZEC, original inventory and zero NSM transfer. Most lifetime purchase allowance is still occupied, which the report treats as a remaining availability weakness.

The main agent reran full locked Rust tests, formatting, strict Clippy, exact signed vectors and package checks after adding the reserve study. The Rust suite remains at 71 passing integration tests. The new figure was generated with Matplotlib 3.10.8 and visually inspected; the PDF was rebuilt with Giacomo Barbieri's attribution and its reserve section checked. Oracle error, fundamental token losses, unmodeled backing and trader behavior, native authentication and shutdown disposition remain explicit limitations.

## Lifecycle and retained-funding policy

The [lifecycle wrapper](reserve-lifecycle.md) embeds immutable configuration, records canonical observation content, requires prepared block context and rejects a second funding credit at the same height. Its allocation budget counts initial cash and later retained credits. Gross epoch limits continue to apply; the original reserve studies remain unchanged as v1 baselines.

Closure burns actual held tokens against explicit bad debt and transfers reserve ZEC to the NSM bucket. Residual tokens remain available through a paid, oracle-free descending-price sale. Active principal is unchanged. A native shutdown, funding schedule, signature verifier and vault/reserve integration have not been implemented by this Python wrapper; the [Rust contract](reserve-rust-contract.md) specifies that next milestone.

Independent review rejected free holder distributions after demonstrating a mint-hold-repay windfall. The reviewer also reproduced a functional outage-clock bug: always-stale reports with advancing sequences could keep postponing closure. The corrected implementation preserves the first uninterrupted stale height. Twenty-two lifecycle tests and three independent review tests pass, including 12 age/grace configurations and a discrete-auction-price counterexample. The independent full Python run passed all 96 tests and reproduced the numeric cases.

The main agent ran the full verification script successfully: 71 Rust integration tests, 96 Python tests, all numeric/vector comparisons, formatting, strict Clippy and package checks. The rebuilt 12-page whitepaper credits Giacomo Barbieri. Residual economics remain explicit: within-epoch quota occupation, oracle-triggered closure, low auction proceeds, unsold inventory and returning cash to NSM while bad debt may remain. This receipt establishes reference arithmetic and observed checks, not production readiness or guaranteed price support.

## Signed Rust reserve integration

The optional reserve now shares the authenticated vault ledger, block overlay and combined invariants. The new chain domain binds all reserve parameters, donor/seed and funding schedule; the legacy constructor and signed scenario remain unchanged. Initial reserve cash is an actual fixture transfer, initial inventory is zero, and every scenario obtains tokens through collateralized vault issuance. Funding is deterministic synthetic regtest issuance, with no user credit action and no approved native allocation.

The complete script passed 102 Rust integration tests and all 96 existing Python tests. Both signed examples and all mathematical-study JSON/CSV outputs reproduced exactly. Formatting, strict Clippy, doc tests, package links/credential patterns and whitespace checks passed. The PDF rebuilt successfully; its author metadata and funding/reserve/shutdown pages were inspected.

The independent [reserve review](reserve-adversarial-review.md) added 19 signed attack tests. It identified a low-severity panic in the public detached-snapshot auction helper when its copied configuration was malformed. The helper now validates that configuration; the regression returns errors and confirms the live state remains unchanged. The issue was found by source inspection, without a recorded pre-fix execution. No other exploitable accounting or authorization defect was found in the reviewed paths; this was not exhaustive exploration or a production audit.

Three checked-in signed scenarios produce these final model checkpoints:

| Scenario | Result | SHA-256 checkpoint |
|---|---|---|
| Borrow, reserve round trip, repay | Supply and bad debt both zero | `181ac4b3a3f3d4178c9f93aa15ff962ea682e3ab6dc33c22e3c54022457c8efb` |
| Crash and partial terminal coverage | 400-token burn leaves 123.809524 zUSD bad debt | `de90497f37dbc6021589e83f13414777b95f1ab053e566dda610d0cfab3ff46c` |
| Uncontested floor auction then repayment | 400 tokens sell for 400 zatoshi; principal repaid | `564adb10876508e93e304deade3d191dcce75392ae843ccfe67f4c39e5bbb26a` |

The adapter tests restore an exact parent and replay closure, funding and auction fills, reject stale sibling commits, and reject late failures without partial state changes. This is in-memory model evidence, not a native node reorg or restart test.

The terminal burn precedes user transactions and happens once; later liquidations can create bad debt while auction inventory remains. Reserve inclusion-age checks cannot detect a quorum re-signing stale market data, and legacy vault operations retain their own expiry rule. Cash returned to NSM may leave debt uncovered, reserve purchases can buy impaired tokens, and poor auction competition can reward borrowers. The signed ledger makes these outcomes executable and measurable; it does not establish an economic equilibrium, a dollar peg or production readiness.
