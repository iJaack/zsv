# ZSV research prototype

This repository contains protocol research and a regtest-only executable model. Native Zebra support remains proposed. Use simulated funds only.

- Preserve the single stablecoin asset boundary. Do not add asset registration, generic issuer keys, arbitrary asset identifiers, a VM, a bridge, or external custody.
- Keep prototype behavior, proposed Zebra integration, and future shielded designs explicitly separate.
- All monetary arithmetic uses integer units and checked bounds. Accepted transactions must preserve the documented supply, debt and ZEC invariants atomically.
- Add regression tests for adversarial findings. Reject mainnet and public-testnet execution in the model.
- Never use real funds, production keys, private conversation context or credentials in fixtures or documentation.
- Public forum content is a draft. Publication, upstream submissions and deployments require explicit authorization.
- Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` before claiming Rust validation. Record scope and failures honestly.

## Writing

Apply [Peter Yang's no-ai-slop guide](https://github.com/petergyang/no-ai-slop/blob/000650b156983f5159695b441477f4e63b25dc85/skills/no-ai-slop/SKILL.md) and its [review checklist](https://github.com/petergyang/no-ai-slop/blob/000650b156983f5159695b441477f4e63b25dc85/skills/no-ai-slop/eval.md) to documentation, drafts and replies. Cut filler, rhetorical contrasts and repeated caveats. Use concrete mechanisms and direct verbs. Preserve equations, claims, source evidence, authorship and material limitations.

## Code cleanup

Use the [AI Slop Cleaner workflow](https://buildwithclaude.com/skill/ai-slop-cleaner) for cleanup requests. Its [source instructions](https://github.com/yeachan-heo/oh-my-claudecode/blob/main/skills/ai-slop-cleaner/SKILL.md) require a bounded plan, behavior checks before edits, small deletion-first changes and a separate review. Preserve public APIs, signature bytes, checkpoint formats, error behavior and protocol rules. Reuse existing helpers and avoid new dependencies.
