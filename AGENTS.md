# ZSV research prototype

This repository contains protocol research, a signed regtest model and offline development profiles linked to Zebra. The collateral profile checks serialized fixture UTXOs and vault spends; node enforcement and a native stablecoin value pool remain proposed. Use simulated funds only.

- Treat `zips/draft-ijaack-zsv.md` as the primary candidate consensus specification. Keep proposed ZIP rules distinct from implemented research behavior; do not describe legacy vectors as conformance evidence for unimplemented wire/oracle/retirement rules. Public payments are the initial scope; shielding is a follow-up. Keep ZIP status Draft and numbering unassigned until the appropriate process changes them.
- Preserve the single stablecoin asset boundary. Do not add asset registration, generic issuer keys, arbitrary asset identifiers, a VM, a bridge, or external custody.
- Keep the account model, Zebra-linked block experiments, proposed native enforcement and future shielded designs explicitly separate. A data commitment in a coinbase output does not lock collateral or create a native stablecoin.
- The collateral profile uses a trusted funding bootstrap, restricted V4/P2PK signatures and its own mandatory vault spend guard. Its vault script is anyone-can-spend on unmodified nodes. Preserve explicit Ed25519-to-secp256k1 owner bindings, exact-parent checkpoints, all-input checks and per-transaction UTXO/ledger reconciliation. Reject reserve state and synthetic funding until a native reserve cash lifecycle exists.
- All monetary arithmetic uses integer units and checked bounds. Accepted transactions must preserve the documented supply, debt and ZEC invariants atomically.
- Add regression tests for adversarial findings. Reject mainnet and public-testnet execution in the model.
- Never use real funds, production keys, private conversation context or credentials in fixtures or documentation.
- Public forum content is a draft. Publication, upstream submissions and deployments require explicit authorization.
- Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` before claiming model validation. Run `bash zebra/verify-dev.sh` for changes to the Zebra-linked runner; its targeted tests use nextest. Record scope and failures honestly.

## Writing

Apply [Peter Yang's no-ai-slop guide](https://github.com/petergyang/no-ai-slop/blob/000650b156983f5159695b441477f4e63b25dc85/skills/no-ai-slop/SKILL.md) and its [review checklist](https://github.com/petergyang/no-ai-slop/blob/000650b156983f5159695b441477f4e63b25dc85/skills/no-ai-slop/eval.md) to documentation, drafts and replies. Cut filler, rhetorical contrasts and repeated caveats. Use concrete mechanisms and direct verbs. Preserve equations, claims, source evidence, authorship and material limitations.

## Code cleanup

Use the [AI Slop Cleaner workflow](https://buildwithclaude.com/skill/ai-slop-cleaner) for cleanup requests. Its [source instructions](https://github.com/yeachan-heo/oh-my-claudecode/blob/main/skills/ai-slop-cleaner/SKILL.md) require a bounded plan, behavior checks before edits, small deletion-first changes and a separate review. Preserve public APIs, signature bytes, checkpoint formats, error behavior and protocol rules. Reuse existing helpers and avoid new dependencies.
