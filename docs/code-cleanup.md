# Code cleanup

This pass uses the [AI Slop Cleaner](https://buildwithclaude.com/skill/ai-slop-cleaner)
workflow. Scope: the private numeric signing encoder in `src/lib.rs` and focused
regression coverage in `tests/cleanup.rs`.

## Plan before code

1. Protect transaction and oracle signing bytes with literal byte fixtures. Cover
   every action tag, both asset tags, unsigned 64-bit little-endian fields, keys,
   chain domains and action order. Run these tests before changing the encoder.
2. Remove repeated adjacent numeric encoding calls by extending the existing
   private `number` utility to encode an ordered slice. Keep all action branches
   and tags explicit. Do not introduce a new public API or dependency.
3. Replay the unchanged signed scenario vectors, compare generated scenario
   output byte for byte, and run the repository's Rust checks.

Preserve public types and methods, canonical snapshot/checkpoint formats,
cryptographic checks, error semantics, monetary bounds and protocol rules.

The adapter's height-only snapshot clones are outside this pass: removing them
would require a public accessor or a change to parent checks. The package walker
also stays unchanged; pruning ignored directories would add traversal logic
unrelated to the numeric duplication.

## Evidence

Before edits, `scripts/verify.sh` passed with Rust 1.90: 69 tests, doc tests,
formatting, Clippy with warnings denied, exact scenario comparison, package check
and whitespace check. The baseline log is `/tmp/zsv-cleanup-baseline.log`.

A separate reviewer inspected the plan, source diff and literal-byte tests, then independently ran the complete verification script. The review passed with no required follow-ups.

The two new literal-byte tests passed against the original encoder before the
implementation edit. They also pass after replacing the encoder's adjacent
numeric calls with ordered slices. Implementation code changed only in `src/lib.rs`
(15 added lines, 23 removed); no action branch, tag or validation check changed.

After the edit, `bash scripts/verify.sh` passed with Rust 1.90: 71 tests, doc
tests, formatting, Clippy with warnings denied, byte-for-byte comparison against
the unchanged `examples/scenario-output.json`, package check and whitespace
check. The verification log is `/tmp/zsv-cleanup-code-verification.log`.

The remaining risk is accidental signing-encoding drift, covered by the literal
fixtures, existing signed scenario replay and exact scenario comparison.

The reviewer confirmed unchanged Cargo manifests, lockfile and signed scenario vectors. Resource limits, native Zebra integration, oracle governance, peg mechanisms and loss recovery remain outside this cleanup.
