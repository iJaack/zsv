# Combined stress and Zebra-linked harness review

This independent review covers the signed economic stress runner and the isolated crate importing Zebra at `5faf8a413f0d30a6e93115f3d16af1e229fd7858`. It inspected source, added ten adversarial harness tests, and independently recomputed the monetary results of all 351 stress cases. It does not cover a patched node, native collateral enforcement or production economic parameters.

## Archive-authentication finding

**Low, fixed: a trusted selected tip did not authenticate the complete branch archive.** The first replay API accepted an externally saved tip hash. Consider an archive that accepts sibling blocks A and B, then selects A. Removing B from the journal and reducing its count preserves the selected tip and A's valid history. Replaying against A alone could therefore succeed while silently losing B. The selected state was not forged; the defect concerned the promised recovery of all retained branches.

The implementation now requires a separately trusted `JournalAnchor` containing the selected tip and the SHA-256 digest of the complete journal. Replay checks the digest before parsing, then revalidates every envelope from the trusted model genesis. The regression removes the unselected sibling while retaining A and confirms rejection; replaying the original journal restores the exact archive.

This issue was identified from source before the fix. No pre-fix failing execution was recorded; the evidence is source inspection and the passing post-fix regression, not a recorded red/green run. The digest must be obtained from a trusted source outside the journal. Replacing both an untrusted journal and its alleged anchor cannot be detected by an unkeyed hash alone.

## Native-data tests

All ten independent tests in `zebra/dev-harness/tests/review.rs` passed under Cargo nextest. Targeted strict Clippy passed. The implementation agent's complete harness suite contains these ten tests plus five implementation tests and passed its verification script.

The independent tests establish the following observed behavior:

- Mainnet, public Testnet and nonzero-height model genesis states are rejected.
- Replacing a committed model payload with a different, validly signed payload fails.
- Altered coinbase bytes fail the native transaction Merkle check; recomputing that root still fails when the coinbase digest does not match the payload.
- A correct native data commitment does not substitute for valid model signatures.
- Native coinbase height must match the committed model height.
- Oversized declared lengths, excessive envelope bytes, noncanonical JSON, truncation and trailing bytes fail without state changes.
- A child uses the exact native parent and that parent's model checkpoint, even when another sibling is the selected tip. Detached returned states cannot mutate archived states.
- Duplicate blocks, unknown parents and rewritten parent checkpoints are rejected atomically.
- Journal recovery rejects a changed model genesis, wrong external tip or digest, corrupted bytes and deletion of an unselected sibling.

The code also rejects growth beyond its aggregate journal-byte limit before accepting a candidate. Thus the per-envelope and branch-count limits cannot produce an accepted archive that already exceeds the export limit. These are offline experiment bounds; full-state cloning and retained vault history still require a production memory and storage-cost design.

The native commitment is real data binding: a zero-value V1 coinbase output commits the model domain, exact parent checkpoint and canonical payload; its transaction hash and Merkle root bind that output to the header. It remains a data-only construction. The fixture does not lock native ZEC, mint a native asset or enforce native proof, script, subsidy, activation or proof-of-work rules. V1 fixtures passing the codec are not evidence of validity under current Regtest consensus.

## Independent mathematical checks

`analysis/check_combined.py` recomputes the documented fixture arithmetic using Python integers and `Fraction`, independently of Rust ledger execution and the golden-file comparison. It verifies every expected parameter combination is present exactly once and checks all 351 result rows:

- 324 liquidation cases: eligibility, affordable burn, terminal inventory coverage, remaining bad debt, active principal, supply, NSM cash, principal/bonus shortfalls and exposure duration.
- Six stale-report cases: ZEC transferred, its assumed true-price dollar mark and the conditional advantage when tokens are valued at one dollar.
- Twelve funding cases: retained budget, indivisible purchase capacity, cumulative allocations, inventory, spending, NSM cash and exact public-cash reconciliation.
- Nine auction cases: the first affordable price under the assumed buyer rule, proceeds, residual inventory, active principal and borrower profit after full repayment and collateral return.

All checks passed. Source review also confirmed that every stable unit originates in a signed collateralized issuance, and that expected failed orders check the precise rejection and unchanged parent state. An initial fixture helper hardcoded epoch zero despite a one-block funding epoch; both implementer and reviewer noticed it during development. It was corrected before the published results were generated.

The important economic outcomes remain adverse under correct accounting. A live underwater vault can show zero recorded bad debt when no liquidator can execute. Closing-block liquidation can create new debt after the one-time reserve burn. A borrower bidding alone at the auction floor can complete the loan cycle with 3.979996 ZEC profit before costs. Validly re-signed stale market prices can transfer value through the reserve's standing quotes. None of these observations establishes an equilibrium, a peg-recovery probability or an appropriate production parameter.

## Reproduction and scope

Run `python3 analysis/check_combined.py` for the independent arithmetic checks. Run `cargo nextest run --locked --manifest-path zebra/dev-harness/Cargo.toml --test review` for the independent native-data probes. `zebra/verify-dev.sh` runs the full isolated harness checks. The repository verification receipt records the full model and document checks performed by the main agent.

Reviewed SHA-256 fingerprints:

- `zebra/dev-harness/src/lib.rs`: `3aa64aa39ad11ede493def932098930597bf6d652999125b9f4c9c3fb1535a7b`.
- `zebra/dev-harness/tests/review.rs`: `1ee35047f36ed7b042d5ecf132a6f21d605a2a7c9017f0921cd65c14beb19041`.
- `zebra/dev-harness/Cargo.lock`: `9413826fb02dc3ccba2537c576587a44c9393d1081f8b61dc17905829738baa4`.
- `examples/combined_stress.rs`: `412f0decec25ced6807d9e0fb92c5c3aa8d27ea77e43549f663e4e098f7c41a2`.
- `analysis/combined-results.json`: `6fb62bb18a5e5fa486cfdef44f065fe63a05c200d02a342cae2b8bec96c822f9`.
- `analysis/check_combined.py`: `ebb1a05223dbb73f7245607cc8f9522f2a689f258e48f042e852c48e72a35107`.

No remaining exploitable authorization or accounting defect was found in the inspected paths. This review did not perform fuzzing, exhaustive state-space exploration, a dependency audit, native consensus verification, disk-failure testing or market calibration. Input replay and exact archive comparison do not establish crash-safe storage, finalization or chain-work fork choice.
