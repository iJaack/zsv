# Research fact-check — 3 November 2026

Reviewed revision: `413fd0f`. This pass checks source claims and reproducible research results. It does not certify a production stablecoin, native ZIP implementation or dollar peg. Corrections from this pass are included with this record.

## Primary sources checked

The ZIP repository returned `ff594de6e95bb9e9e7d2ace09510281531c30447` as `main`. The [source register](sources.md) links the immutable research baseline. The following claims were rechecked against primary text at the refreshed ZIP pin:

| Claim | Evidence and result |
|---|---|
| General ZSA transfer and issuance are draft proposals | ZIPs 226/227 both have Draft headers. ZIP 227 includes arbitrary custom assets and issuer authority. This does not establish network deployment. |
| Historical ZSA v6 cannot be reused as the ZSV allocation | ZIP 230 is Withdrawn. ZIP 229 defines v6 with group ID `0xD884B698`. ZIP 258 is Final and specifies NU6.3 branch `0x37A5165B` and Mainnet height 3,428,143. |
| NU7 scope excludes general ZSA issuance | ZIP 259 is Proposed, lists its deployed ZIPs, adds no transaction format and excludes v4 after activation. It specifies Mainnet 3,543,405 and branch `0x77190AD9`. No live tip was queried. |
| Funding streams are 8% ZCG and 12% deferred | ZIP 214 tables specify numerators 8/12 over 100 and map `FS_CCF_H3` to `DEFERRED_POOL`. Its revision status and abstract differ; do not infer activation from the header alone. |
| The 78,750-ZEC disbursement is distinct from ongoing accrual | ZIP 271 specifies that amount and a separate lockbox disbursement; its header is Proposed. The text supplies no ZSV spending authority. |
| NSM and faster-block assumptions need explicit scope | ZIPs 237/218/259 are Proposed at this pin. Research excludes reserve-dependent NSM reissuance; idealized daily issuance is not a measured emission stream. |
| Ed25519 validation cannot silently substitute a strict library mode | Final ZIP 215 permits noncanonical point encodings and requires its specified cofactored equation. The candidate's canonical roster restrictions are separate. Current fixture checks cover canonical signatures only. |
| ZIP submission and release are separate | ZIP 0 permits an initial Draft, editor-assigned number and MIT license. Significant security/privacy implications require qualified independent review before Released status; a complete implementation generally need not precede Proposed. |

[Upstream issue 2443](https://github.com/zcash/librustzcash/issues/2443) describes TZE as development-only and not planned for deployment. It was opened and closed on 22 June 2026. An issue closure alone does not prove a particular removal patch; the whitepaper now quotes the supported characterization rather than treating closure as implementation evidence.

The [ZCG proposal](https://forum.zcashcommunity.com/t/zcg-gets-too-much-zec-heres-our-plan-to-fix-that/58566), first posted 29 October 2026, states approximately 109,000 ZEC under the existing horizon, approximately 47,000 under the taper, approximately 3% monthly reductions, and a separate conditional 36,200-ZEC treasury contribution. Thus 10% of the rounded future-funding difference is approximately 6,200 ZEC. This verifies what the authors proposed, not their underlying forecast or a payable schedule. No source spreadsheet or finalized schedule was obtained.

The [programmability discussion](https://forum.zcashcommunity.com/t/private-programmability-in-zcash-research-results-and-community-discussion/48016), first posted 20 June 2024, identifies a Foundation-funded research project. Its proposed mechanisms are research recommendations, not deployed interfaces. [QEDIT's page](https://qed-it.com/zsa-hub/) says activation remains a community decision; its implementation/audit statements are attributed vendor statements, not audits performed by this project.

## Source and executable checks

The local Zebra checkout is clean at `5faf8a413f0d30a6e93115f3d16af1e229fd7858`. Source inspection confirms the transaction wrapper and dependency-level parser, parent-specific contextual entry, transparent-spend boundary and direct-finalization path named in the source register. Native implementation remains proposed. Upstream has advanced to `b2fd4ce06b3469259683650c173d7689ced8b2b0`; this pass does not migrate the source map or dependency pin.

Using Rust 1.90, the following completed successfully:

- `bash scripts/verify.sh`: 110 Rust tests (104 integration plus six combined-study tests), doc tests, formatting, strict Clippy, exact signed scenarios, 351 combined cases, 96 Python tests and all checked-in study results.
- `bash zebra/verify-dev.sh`: 39 Zebra-linked tests, formatting, strict Clippy and both exact scenario comparisons. These are offline fixture checks, not a patched-node test.
- `bash scripts/verify-zip.sh`: 54 financial cases, seven unsigned wire examples, canonical oracle signature/context checks and 13 block-selection cases.

Additional exact arithmetic confirmed the 6,200-ZEC funding subtraction, the 20% fall from 250% collateralization to 200%, the 105% liquidation coverage threshold, the 85.714285-token liquidation / 14.285715-token loss example, and the proposed u128 product bound. The independent Fraction checker reproduced all 351 combined results. Its independence is from the ledger's arithmetic implementation, not an independent professional reviewer.

The controller bounds `0<k<2` without delay and `0<k<1` with one-epoch delay follow from the stated linear recurrences. They require fixed demand/depth, consistent participant response and no binding constraints. They establish no corresponding range for a real unmeasured market.

## Findings and evidence limits

The whitepaper now describes native verification prospectively, identifies the model's two-key compromise separately from the candidate's five-key compromise, and labels the economic review as coding-agent work. The ZIP remains Draft and unnumbered. Public payments, parent-oracle pricing, dedicated collateral accounting and uncompensated retirement remain proposed consensus rules.

The oracle-renewal finding is a specification-level interaction: immediate activation changes the signed report ID before pending transactions execute. Parent-selected pricing removes that same-block supersession and adds one block of delay. The fixture checks and recorded mutation tests support this correction; full signed transaction and native block tests remain outstanding. The payout finding was a missing assertion in an unsigned example, not evidence of a consensus theft exploit.

The research does not establish real market depth, a stable dollar price, safe risk parameters, independent reporter operations, recapitalization incentives, future governance approval, live activation or production security. Coding-agent adversarial reviews are preliminary evidence. Qualified expert review, full native implementations, independent conformance and resource benchmarks remain required before deployment claims.
