# ZSV ZIP submission candidate

The primary proposal is [draft-ijaack-zsv.md](../zips/draft-ijaack-zsv.md), authored and owned by **Giacomo Barbieri <hey@jaack.me>**. Public stablecoin payments are the confirmed initial scope; shielding belongs in a follow-up specification. The draft is unnumbered and has not been submitted upstream.

## Process and scope

The draft follows [ZIP 0](https://github.com/zcash/zips/blob/ff594de6e95bb9e9e7d2ace09510281531c30447/zips/zip-0000.rst) and the [Markdown template](https://github.com/zcash/zips/blob/ff594de6e95bb9e9e7d2ace09510281531c30447/zips/zip-template.md), reviewed on 30 October 2026. This process pin is separate from the consensus/Zebra research baseline. The ZIP uses an alias filename, Draft status, Consensus category and MIT text license. `Created` remains a placeholder for the actual submission date. No discussion URL, editor assignment or review approval is invented.

ZIP 0 generally permits Proposed status before the reference implementation is complete. It also requires a sufficiently specified proposal, the applicable editor/community process and qualified independent security/privacy review for release of a proposal with significant implications. The coding-agent reviews here are preliminary review and test evidence, not that expert review.

The core contains vaults, one public stable asset, oracle rules and loss accounting. It has no Dev Fund allocation, reserve or general token issuer. A reserve/funding proposal can be considered separately without becoming an implicit dependency of the core. The ZIP specifies behavior across implementations; Zebra is the intended implementation work, not the definition of consensus.

## Concrete choices made

| Area | Proposed rule |
|---|---|
| Native assets | Existing ZEC plus implicit zUSD fields; no arbitrary asset selector |
| Collateral | Dedicated ZEC vault value pool, not an ordinary spendable marker output |
| Payments | Public fixed-key zUSD UTXOs; existing ordinary ZEC transactions remain available |
| Transaction shape | New unallocated version; one NONE, OPEN, UPDATE, CLOSE, LIQUIDATE or RETIRE_BAD_DEBT operation; exact byte and auth grammar |
| Authorization | Compressed secp256k1 ownership, strict DER/low-S signatures over effects and actual resolved state; network/upgrade binding |
| Vault economics | 250% borrowing/withdrawal ratio, strict-below-200% liquidation, 5% bonus, 100-zUSD minimum nonzero debt, 100,000-zUSD initial supply cap |
| Oracle | Fixed upgrade-selected seven-key roster; five matching signatures; anchored observation valid for twelve blocks; exact report-ID binding |
| Losses | Explicit bad debt stops borrowing; holders may burn existing zUSD against it without compensation; no administrator reset or redemption promise |
| Resources | 65,536 bytes per new transaction; 256 such transactions, 1,024 secp256k1 auth slots and 4,096 new payment outputs per block; at most seven oracle signatures |

These are draft choices, not empirically calibrated risk parameters or allocated network constants. Block budgets bound specified work but do not establish measured production cost or eliminate state-growth attacks.

## Relationship to the research implementation

The existing Rust implementation and its vectors are preserved. Its signed account API, 2-of-3 oracle, inclusion-relative report lifetime, V4/Sapling collateral fixtures, JSON envelopes, key mapping and zero-fee fixture policy are not normative ZIP encodings. The new candidate instead uses the native format, oracle and authorization rules above. It also introduces uncompensated bad-debt retirement, normalized single-vault transitions and exact owner payout constraints.

The previously published tests remain evidence for their original scope. They do not establish that the new ZIP format or oracle is implemented. The [whitepaper](whitepaper.md) retains the research results and now identifies this distinction explicitly.

## Verification and review

Run `bash scripts/verify-zip.sh`; dependency and scope details are in the [vector README](../zips/draft-ijaack-zsv/README.md). The main agent ran the complete script successfully:

- 54 financial cases passed independent Fraction arithmetic and u128 range checks. A deliberately corrupted expected mint fails the checker.
- Seven unsigned wire/context/hash examples reproduced exactly, covering all six operations and an oracle effect body. A separate byte-cursor review independently parsed every example, recomputed its hashes/role digests and checked the UPDATE, CLOSE and liquidation payouts.
- The oracle fixture verified seven canonical signers, four quorum subsets, sixteen rejection probes and twelve numeric/ancestry context cases, with exact JSON reproduction.
- Package screening passed for 114 files; relative links and bounded credential patterns passed, as did the whitespace check.
- The pinned ZIP repository's actual `render.sh --pandoc` rendered the candidate successfully. The resulting HTML contains the author/contact, resolves its internal links and has four tables with consistent row widths. The MultiMarkdown path was not run.
- The whitepaper rebuilt as a 14-page PDF, research draft 0.3. Giacomo Barbieri's author metadata and the rendered ZIP scope section were checked.

The Rust sources and existing vectors did not change, so their previously recorded test results were preserved rather than presented as new ZIP conformance tests.

Separate coding agents drafted the financial, wire and oracle rules and reviewed the assembled proposal for process and cross-section consistency. An independent financial review found an impossible test prestate implying a separate 50-zUSD vault below the 100-zUSD minimum. The fixture was corrected and the checker now verifies that other live principal is either zero or at least the minimum. This was a fixture defect, not a demonstrated financial formula exploit.

Review of the assembled specification identified two validity ambiguities: missing explicit whole-block atomicity across the new pools/state, and an overly broad phrase about monetary aggregate bounds that could be read as a cap on cumulative same-block turnover. The final draft requires exact-parent execution and complete atomic rollback/finalization/restart equivalence, and scopes amount bounds to individual transactions and persistent balances. It also makes the trusted network input to transaction hashing explicit and includes the vault-pool conservation invariant. No unauthorized mint or collateral-payout path was identified in the reviewed cases; this is not an exhaustive security result.

The independent process review found the author metadata, public-first privacy disclosure, licensing and Draft status consistent with the pinned ZIP process. It confirmed that new-format/oracle/retirement rules are distinguished from the tested prototype and that qualified expert review remains outstanding.

## Remaining submission and deployment work

Before upstream submission: collect public feedback using the [forum draft](forum-post.md), supply the actual submission date and discussion URL, rerun the checked upstream renderer after final metadata changes and package auxiliary files for the ZIP repository. Existing repository publication does not imply that any of these external actions occurred.

Before deployment: allocate transaction/group and upgrade identifiers, choose Mainnet/Testnet activation and actual reporter operators/keys, implement the rules, generate complete signed/native block vectors independently, measure costs, and obtain qualified consensus/cryptographic/privacy and economic review. Reporter source quality, willingness to absorb bad debt, liquidation-market depth and a dollar peg remain external economic risks.

The next engineering work should follow the candidate rules and expose disagreements with them. Finishing a node implementation is not a condition for beginning community discussion of the draft.

## Source fingerprints

SHA-256 of the reviewed candidate and its conformance material:

- `zips/draft-ijaack-zsv.md`: `d91ca1494b95cda9bfcb7a59518e820fd6849b43dbdb9028b9a9e4475750383a`.
- `zips/draft-ijaack-zsv/check_financial_vectors.py`: `d5dd93fc554af2a94f459cef1c59826e5fc9ba74bb65bf6e11d075af81f0cbf4`.
- `zips/draft-ijaack-zsv/financial-vectors.json`: `2ca1b02eda82c065672a768cabcef6081c511d6398a32bfecaf0b763f3eb7ef9`.
- `zips/draft-ijaack-zsv/check_wire_vectors.py`: `30f3b6d1e7a9fb0ce7cc927138127b875d6c1ccf5e992b1f0e8e32ecb9f9289f`.
- `zips/draft-ijaack-zsv/wire-vectors.json`: `5b38eabb68a6a486b3e8ca985f4c471e2508bd9e145c41dfac5d648d95a4d5cd`.
- `zips/draft-ijaack-zsv/check_oracle_vector.py`: `0a7acae381c0f1d2f71597ea0d2006084510b39fcab07fc1d94470a387dbfe14`.
- `zips/draft-ijaack-zsv/oracle-vector.json`: `0af82f53ed8c545b7d9e5e81dc5e17812e21cf2e3d88e77cfd16437dd35d9063`.
