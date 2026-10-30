# Zebra-linked development harness

The isolated harness in [zebra/dev-harness](../zebra/dev-harness) connects the signed ZSV ledger to the actual Zebra library at commit `5faf8a413f0d30a6e93115f3d16af1e229fd7858`. It runs offline with simulated balances. Its purpose is to test payload binding, native block identity, parent selection and replay before implementing a new native asset format.

A commitment in a transaction can bind data to a block. It does not encumber ZEC or authorize a new asset. The harness keeps that distinction: native block parsing and hashing come from Zebra; vault balances, reserve funding and stablecoins still come from the research ledger. Existing Zebra nodes do not enforce ZSV rules by recognizing this commitment.

## Integration boundary

The development carrier permits exactly one V1 coinbase transaction with one input and one zero-valued output. Its fixed script commits the canonical signed payload, model chain domain and parent checkpoint. Validation checks that carrier, native transaction Merkle root, previous block hash and coinbase height, then executes the payload against that parent's state. V1 transaction hashes bind all coinbase bytes; the single-transaction restriction avoids duplicate-transaction Merkle ambiguity. The [crate README](../zebra/dev-harness/README.md) specifies the encoding and byte bounds.

The root is pinned to Zebra's regtest genesis. Native block bytes have no network tag; a caller-supplied Regtest label alone cannot identify a chain. The configured root and checked ancestry supply the harness's chain context. Mainnet and public Testnet are rejected by its API.

The offline fixtures are serialized native block structures. They are not mined, fully consensus-verified blocks. The harness does not run Equihash, difficulty, subsidy, ordinary input/script or shielded-proof validation. The model's funding schedule remains synthetic, with no claim on the coinbase output or native NSM subsidy.

## Branch state and replay

Each accepted branch uses its native block identity and the exact parent's model state. Competing children must start from that same parent; neither may inherit changes from the other. Reserve funding, epoch counters, oracle state, inventory, shutdown burns, auctions and wallet nonces travel with the branch.

A rejected block leaves both the branch archive and selected tip unchanged. Replay requires the original model genesis and a separately trusted receipt containing the selected native tip and SHA-256 of the complete journal. Checking only the tip originally allowed an unselected sibling to be removed; independent source review identified that gap and the full-journal digest closes it. A digest stored only alongside attacker-controlled bytes supplies no independent trust. Replay checks the external digest, then reruns parsing, commitment checks and signed state transitions. It never imports asserted post-state balances.

This harness uses explicit branch selection for experiments. It does not implement consensus fork choice, Zebra's non-finalized/finalized database, atomic disk commits or crash recovery. Replaying an exported history is distinct from proving database durability.

## Threats to test

- Substitute the model payload, chain domain or parent checkpoint under an unchanged native header.
- Change transaction bytes while retaining an old Merkle root; exploit duplicate transaction identities or commitment outputs.
- Supply a wrong parent, height or network; transplant a branch onto another model genesis.
- Append trailing bytes, use noncanonical encodings or exceed input limits before parsing.
- Fail after a valid reserve trade, funding credit or shutdown; verify complete rollback.
- Replay a journal with altered inputs, missing parents or an invalid selected tip.
- Import a snapshot containing invented balances instead of re-executing signed inputs.

The [development review](development-review.md) and [verification receipt](verification.md) record tested cases and observed limits. The harness caps encoded input, retained envelopes and block count, but retains a full model state per block. These bounds have not been benchmarked as a production memory or CPU budget.

Run `bash zebra/verify-dev.sh` with Rust 1.90 and Cargo nextest installed. It runs the isolated crate's nextest suite, formatting, strict Clippy and exact scenario comparison against its separate lockfile. The initial dependency fetch needs network access; no node or peer service starts. The three signed reserve scenarios retain their original model checkpoints after native commitment validation and journal replay.

## Remaining native work

The production design still needs a reviewed native transaction extension in librustzcash, vault collateral encumbrance enforced for every spend path, fixed-asset input/output conservation and proof statements, native authorization digests, semantic and contextual node checks, approved coinbase/NSM rules, durable branch state, mempool handling and a wallet flow. Those changes must retain ordinary Zcash validation and fail closed on unsupported networks and upgrades.

The source-pinned [integration proposal](zebra-integration.md) maps those changes to upstream modules. No upstream PR or node deployment accompanies this development harness.
