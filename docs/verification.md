# Verification receipt

Status: collateral and ZIP-candidate checks completed on 30 October 2026. Public delivery is authorized; earlier deliveries are recorded below and each source publication is recorded in the repository history.

## Scope

The checks cover the signed account ledger, its contextual adapter, the combined economic study and offline profiles linked to the actual pinned Zebra library. The first profile binds model payloads to native fixture blocks. The new collateral profile verifies serialized ZEC UTXOs, restricted native signatures and per-transaction vault accounting. Its guard is not installed in Zebra; a native stablecoin value pool, full node consensus and real funds remain outside scope. The forum post and upstream proposal remain drafts.

The native integration proposal uses Zebra `5faf8a413f0d30a6e93115f3d16af1e229fd7858` and the ZIP pin recorded in `sources.md`. The initial research inspected those sources. The development crate now compiles zebra-chain from that pin; no patched zebrad node was built.

## Reproduction

```sh
bash scripts/verify.sh
bash zebra/verify-dev.sh
bash scripts/build-whitepaper.sh
```

The model script runs locked Cargo tests, doc tests, formatting, strict Clippy, signed vectors, combined stress reproduction, independent Fraction checks, Python mathematical tests, earlier numeric studies and package checks. The Zebra script uses Cargo nextest for its separate crate, then formatting, strict Clippy and exact comparisons for both scenario binaries. The paper script uses Graphviz, Pandoc and pdfLaTeX. Rust 1.90 is selected by `rust-toolchain.toml`; Cargo may need a network connection for its initial dependency download. The mathematical study needs Python 3.10 or later; Matplotlib is optional for figure regeneration.

## Observed final results

The main agent ran both model/development verification scripts successfully after the collateral expiry and monetary-range fixes. Those Rust sources are unchanged in the later ZIP revision. The separate ZIP verification script and document build passed for the candidate; its detailed scope is recorded in [zip-readiness.md](zip-readiness.md). The table separates the reference model from the Zebra-linked test suite; neither is a full node test.

| Check | Observed result |
|---|---|
| Adversarial integration tests | 22 passed |
| Boundary integration tests | 6 passed |
| Signing cleanup integration tests | 2 passed |
| Detached block trace integration tests | 2 passed |
| Economics integration tests | 9 passed |
| Independent reviewer attack tests | 21 passed |
| Signed scenario vector replay | 1 passed |
| Contextual adapter tests | 10 passed |
| Reserve core tests | 8 passed |
| Independent reserve attacks | 19 passed |
| Combined reserve adapter tests | 3 passed |
| Signed reserve vector replay | 1 passed, covering three scenarios |
| Reference Rust integration tests | 104 passed; 0 failed; 0 ignored |
| Combined stress example tests | 6 passed |
| Total reference-model Rust tests | 110 passed |
| Zebra-linked nextest suite | 39 passed: 15 earlier tests, 4 collateral, 6 wire, 13 independent collateral review, 1 replay test covering three collateral scenarios |
| Combined economic study | 351 exact signed-ledger cases; independent integer/Fraction recomputation passed |
| Python mathematical tests | 96 passed: 71 previous, 22 lifecycle, 3 independent lifecycle review tests |
| Numerical study reproduction | Exact JSON/CSV match; funding 10 scenarios / 240 combinations; reserve 11 / 144; lifecycle accounting cases |
| Unit / doc / other example targets | Successful; no additional tests defined |
| Rust formatting | Passed |
| Strict Clippy, all targets | Passed; warnings denied |
| Scenario regeneration | Model vectors, 351-case study and both Zebra scenario outputs match checked-in JSON exactly |
| Package screening | 114 source/artifact files; relative links and bounded credential patterns passed |
| Git whitespace check | Passed |
| Whitepaper | 14-page PDF generated; Giacomo Barbieri attribution, draft 0.3 date and ZIP scope section checked; rendered page inspected |
| ZIP candidate financial vectors | 54 cases; exact Fraction recomputation and u128 bounds |
| ZIP candidate wire vectors | 7 unsigned examples; all six operations and oracle framing |
| ZIP canonical oracle fixture | 7 signers, 4 quorum subsets, 16 rejection probes, 12 context cases |
| ZIP rendering | Pinned upstream `render.sh --pandoc` succeeded; internal links and table structure checked |

The scenario contains a solvent liquidation with supply/bad debt both zero, and a crash case leaving 209,523,810 micro-zUSD as explicit bad debt. Its final model checkpoints are:

- Solvent: `1efc6b48c5a1a5c32035fa3cd43fb8e561c27c3d912ee8ccb802659fe1ece759`.
- Crash: `03afd614880bf7efe186b4bcfb02f70dbf2588d75de5dc6f79f22d1de7a6d5ac`.

The model uses these fingerprints to check repeatable state transitions. A native implementation needs Zcash block hashes and consensus state commitments.

## Source fingerprints

These hashes identify the unchanged collateral source and the current rebuilt whitepaper. ZIP-candidate source fingerprints are recorded in [zip-readiness.md](zip-readiness.md). The adversarial review records the earlier source versions it examined.

- `src/lib.rs`: SHA-256 `0053a1996b70c965c9c6f3f84a45f8625c41016b7be8982666eda974c1c20c55`.
- `src/reserve.rs`: SHA-256 `a219be80b71985a984b84eb626057908a001b538914d9bb9893fe09bd6c07c81`.
- `tests/reserve_attacks.rs`: SHA-256 `7a16cff322c82f34fecdce60acfc64a425ed4152ebe68382114a6d6006ca9208`.
- `examples/reserve-scenario-output.json`: SHA-256 `575a1cea502679d94083240d9ebad7395c647182c729a1ba90d1de2dde649895`.
- `examples/combined_stress.rs`: SHA-256 `412f0decec25ced6807d9e0fb92c5c3aa8d27ea77e43549f663e4e098f7c41a2`.
- `analysis/combined-results.json`: SHA-256 `6fb62bb18a5e5fa486cfdef44f065fe63a05c200d02a342cae2b8bec96c822f9`.
- `analysis/check_combined.py`: SHA-256 `ebb1a05223dbb73f7245607cc8f9522f2a689f258e48f042e852c48e72a35107`.
- `zebra/dev-harness/src/lib.rs`: SHA-256 `87f4bdcd7d780a63869efc6cf5ce5396e3e584fc2169b11c38d08c708595f2af`.
- `zebra/dev-harness/Cargo.lock`: SHA-256 `a1a928e6c2855a2082620c79afed1d617403a1497fcd27cc97cc12f47afe5014`.
- `zebra/dev-harness/scenario-output.json`: SHA-256 `054f3ec60d6b36760207a22b287db0e909cab0e85ff6dd1f5273b1523986ddeb`.
- `zebra/dev-harness/src/collateral.rs`: SHA-256 `1f6f3db24d1109bad4b53c997ec637a45684ac4b94edf60041ff9346c89ed9a9`.
- `zebra/dev-harness/src/collateral_wire.rs`: SHA-256 `edcdffb723fd1822c1428de497c04e0298c666094050f7399a7fd482c557d2dc`.
- `zebra/dev-harness/tests/collateral_review.rs`: SHA-256 `0cf5168bfa5f56726e6998c84d019ed8b086695f6979e44748712257edb80a0a`.
- `zebra/dev-harness/collateral-scenario-output.json`: SHA-256 `db556e81b737670dd181aa4f41ade91db5e059ae3e36651868f7980cd4863733`.
- `Cargo.lock`: SHA-256 `e44aec7ac75310c939371401fd77a3ecf3b2241fa81b50354ad359a5c1f39812`.
- `tests/reviewer_attacks.rs`: SHA-256 `b4b68bfeb05d7c37112421a45cbbf48de00b6addb1e70728c91ab7a459612110`.
- `docs/whitepaper.pdf`: SHA-256 `28226437ef9112c770df8fa232e7d9d4e120b75007d4ecab70573ca0bbf68e5e`.
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

Before deployment, ZSV needs resource pricing, native stablecoin encoding and proofs, collateral guards installed across node acceptance paths, fork and persistence tests, oracle governance, peg mechanisms, recapitalization and shielded payments. The offline profiles have bounded decoding and finite state limits; those do not establish a production resource budget. The protocol cannot prevent off-protocol derivatives or speculation. No production safety, privacy or dollar-peg guarantee follows from these tests.

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

## Combined stress and Zebra-linked development

The [combined study](combined-stress.md) executes 324 liquidation cases, six stale-price trades, 12 funding cases and nine auctions through the signed Rust ledger. Six example tests check selected economic boundaries. The independent [integer/Fraction checker](../analysis/check_combined.py) recomputes all 351 rows from documented assumptions, including parameter-grid completeness. The main agent regenerated and inspected the comparison figure; no market calibration or probability estimate was added. The core vault/reserve source, dependencies and earlier signed vectors retain their previous fingerprints.

The [Zebra development crate](zebra-development.md) compiles against actual zebra-chain 15.0.0 at the source pin using Rust 1.90 and its own locked dependencies. Its verification script passed with nextest 0.9.100: 15 tests, formatting, strict all-target Clippy and exact output for the three existing signed reserve scenarios. The implementation worker ran that script and the main agent inspected its receipt and final source fingerprints. The separate upstream source checkout remains unchanged. No node or peer service was started.

The native codec experiment accepts only its exact V1 coinbase carrier. It verifies a zero-value commitment, Merkle root, coinbase height, native parent and model checkpoint before executing model signatures. Branch selection is manual. Replay reconstructs accepted inputs from a trusted model genesis, selected tip and full-journal digest; it never imports asserted balances. Input, journal and block-count limits do not establish a production resource budget.

Independent review identified that a tip-only replay anchor could miss deletion of an unselected sibling. The fix requires an external digest of the complete journal. The reviewer identified the issue from source semantics; no pre-fix executable failure was recorded. Post-fix regression rejects the deleted-sibling journal and reproduces the complete original archive. The [development review](development-review.md) records the ten independent probes, arithmetic checks and remaining boundaries.

The checks establish deterministic research behavior and data binding through actual native codecs. They do not establish PoW or full transaction consensus, native collateral custody, fixed-asset proofs, coinbase/NSM funding, consensus fork choice, database crash consistency or a stable peg. The economic study retains the closing-block loss-coverage gap and conditional auction subsidy capture as unresolved policy findings.


## Transparent collateral milestone

The [collateral profile](collateral-profile.md) starts from a custom trusted funding block and maintains actual branch UTXOs alongside the model. Native P2PK inputs use secp256k1 signatures and Zebra's fixed Sapling ZIP-243 context. A separate Ed25519 intent binds native transaction bytes, model request, profile domain and parent checkpoint. Every native user transaction is paired; every vault input/output requires an authorized action; each wallet and vault reconciles after every transaction. The shared model trace API preserves earlier signature and checkpoint encodings.

The independent [collateral review](collateral-review.md) contributed 13 adversarial tests and reproduced acceptance of a signed transaction after its native expiry. Candidate-height expiry enforcement fixed it. The core implementer separately reproduced aggregate fixture issuance above MAX_MONEY and added a checked native monetary bound. A zero-expiry helper panic was found and fixed during integration; the independent reviewer inspected that fix without a pre-fix panic reproduction. The receipt distinguishes these evidence sources.

The main agent ran both complete verification scripts successfully: 110 reference Rust tests, 39 Zebra-linked tests, 96 Python tests, the 351-case independent integer/Fraction check, and exact reproduction of all earlier and new numeric/vector outputs. Formatting, strict all-target Clippy, package screening and whitespace checks passed. The rebuilt 13-page paper credits Giacomo Barbieri; its author metadata, date and rendered collateral section were inspected.

Three new signed scenarios conserve 40 ZEC across bootstrap, borrowing and settlement. Repayment and solvent liquidation finish with zero supply and bad debt; crash liquidation leaves 209.523810 zUSD in both. The fixture native final hashes are:

| Scenario | Native fixture block hash |
|---|---|
| Repay and close | `6b5ed003ebf31ec80e49c45cedc9e8f205fc0186df27e4b7f2eda49f8e813b76` |
| Solvent liquidation | `164086e4fff582d393c0c54ae0f3b035bae968ecd71356002f11e693aea3b7dd` |
| Crash and bad debt | `df6851c949839a0397a3395104854efe97dd5010443a27cfdca6d80beeae5101` |

The protocol script remains anyone-can-spend on unmodified nodes. This milestone installs no Zebra service or consensus rule. zUSD remains a model balance; reserve-enabled genesis and synthetic funding are rejected in this profile. Full consensus verification, upgrade-aware signatures, fees, native stablecoin value conservation, all node acceptance paths and durable state remain unfinished. The new profile replays envelopes from its trusted bootstrap and has no authenticated journal or restart implementation.


## ZIP submission candidate

The primary specification is now [draft-ijaack-zsv.md](../zips/draft-ijaack-zsv.md), with Giacomo Barbieri as owner and his approved public contact. The user confirmed public payments first and shielded payments as follow-up work. Draft status and unassigned numbering are preserved; no forum publication or upstream PR occurred.

The candidate specifies a dedicated native transaction/value-pool format, normalized vault operations, a 5-of-7 anchored oracle, uncompensated bad-debt retirement and bounded validation work. These are proposed rules beyond the current Rust model. The separate ZIP checks and review are recorded in [zip-readiness.md](zip-readiness.md). Existing Rust/Python research behavior and vectors were not modified or relabeled as native conformance evidence.

The whitepaper, forum draft, repository navigation and implementation plan now place the ZIP first. The new 14-page paper retains Giacomo Barbieri's attribution. Native implementations, complete signed/block vectors, deployment allocations, reporter operations, economic calibration and qualified independent review remain necessary. A complete Zebra implementation is not a prerequisite for starting ZIP discussion.

## Public-v1 implementation scope cleanup — 30 October 2026

Simplified the README, forum draft, implementation plan and Zebra source map. Removed new shielded circuits, the experimental marker-script guard and reserve/funding state from the core implementation path. The whitepaper's implementation section now matches the candidate's public payments and dedicated collateral pool. Research code, fixtures and normative ZIP rules are unchanged.

`bash scripts/verify-zip.sh` passed all existing candidate checks and package screening; `git diff --check` passed. The paper rebuilt to 14 pages with Giacomo Barbieri's author metadata and the corrected implementation text. PDF SHA-256: `1619bbede35db943f3b040f8f9ee176625d7496cafe24b7379bb8c75277e7150`. Existing Rust test results retain their original scope; no new model validation is claimed for this documentation change.

## Oracle activation and payout checks — 31 October 2026

The candidate now uses the exact parent's committed oracle observation for every operation in a block. An accepted certificate updates state for descendants only. Expiry and domain changes still disable pricing; a new certificate cannot rescue operations in its own inclusion block. The added block of price delay is documented in the ZIP, threat model and whitepaper. No native implementation or change to the legacy Rust model is claimed.

`bash scripts/verify-zip.sh` passed the 54 financial cases, seven unsigned wire cases and canonical oracle checks, including 13 new block-selection cases. Restoring immediate certificate activation in a temporary mutation failed those checks. Mutating the liquidation recipient or shifting one zatoshi between its payouts was missed by the prior fixture helper and rejected after supplying both mandatory payout assertions. These checks operate on fixture helpers, not a native block verifier.

Direct Pandoc rendering passed. The previous temporary upstream renderer was unavailable, so the pinned upstream rendering path was not rerun for this revision. The whitepaper rebuilt to 14 pages; author metadata and the new pricing-delay text were checked. PDF SHA-256: `9d954aa6a645c2d136242d3ff08b6b0ae9a4434da85a81518f310a2f419cc6aa`. Package and whitespace checks passed.
