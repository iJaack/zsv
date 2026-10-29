# ZSV research prototype

This repository contains original protocol research and a regtest-only executable model. It is not an activated Zcash upgrade, a complete Zebra node, a deployed stablecoin, or audited financial software.

- Preserve the single stablecoin asset boundary. Do not add asset registration, generic issuer keys, arbitrary asset identifiers, a VM, a bridge, or external custody.
- Keep prototype behavior, proposed Zebra integration, and future shielded designs explicitly separate.
- All monetary arithmetic uses integer units and checked bounds. Accepted transactions must preserve the documented supply, debt and ZEC invariants atomically.
- Add regression tests for adversarial findings. Reject mainnet and public-testnet execution in the model.
- Never use real funds, production keys, private conversation context or credentials in fixtures or documentation.
- Public forum content is a draft. Publication, upstream submissions and deployments require explicit authorization.
- Run `cargo test --locked`, `cargo fmt --check`, and `cargo clippy --locked --all-targets -- -D warnings` before claiming Rust validation. Record scope and failures honestly.
