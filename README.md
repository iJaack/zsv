# ZSV · Zcash Stablecoin Vault

ZSV proposes locking ZEC to borrow one dollar-targeting stablecoin under Zcash consensus rules. The [ZIP candidate](zips/draft-ijaack-zsv.md), authored by **Giacomo Barbieri**, specifies public vaults and public zUSD payments. It is Draft, unnumbered and not submitted upstream.

The first version needs four things: vault transitions, stablecoin payments, authenticated prices and explicit loss accounting. Shielded payments and any reserve or Dev Fund allocation are separate proposals. There is no arbitrary token issuance, issuer key or general execution environment.

## Start here

- [ZIP candidate](zips/draft-ijaack-zsv.md): proposed consensus rules.
- [Whitepaper](docs/whitepaper.md) · [PDF](docs/whitepaper.pdf): rationale and research results.
- [Threat model](docs/threatmodel.md) · [Economic review](docs/economic-review.md): attacks and unresolved risks.
- [Implementation plan](docs/implementation-plan.md) · [Zebra integration](docs/zebra-integration.md): remaining work.
- [Submission readiness](docs/zip-readiness.md) · [Forum draft](docs/forum-post.md): review and discussion material.

A dollar target is not a redemption promise. False prices, a ZEC crash or insufficient liquidation liquidity can create losses. Recorded bad debt freezes borrowing until holders voluntarily burn enough existing zUSD to retire it. That freeze can last indefinitely.

## What runs today

The Rust executable is an account model using signed requests and simulated funds. Offline Zebra-linked experiments test block binding, branch replay and fixture collateral spends. They do not implement the candidate's native transaction format, 5-of-7 oracle, public zUSD UTXOs or bad-debt retirement.

The experimental collateral script is spendable on unmodified Zcash nodes; its protection exists only inside the test harness. The candidate instead specifies a dedicated vault value pool. Do not use the research software or fixture keys for real funds.

Check the candidate's financial, unsigned-wire and oracle fixtures with Python 3.10+ and `cryptography`:

```sh
bash scripts/verify-zip.sh
```

Run the account model with Rust 1.90:

```sh
cargo test --locked
cargo run --locked --example scenario
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The [conformance README](zips/draft-ijaack-zsv/README.md) and [verification receipt](docs/verification.md) explain the evidence and its limits. No native node implementation or production activation is claimed.

## Supporting research

These experiments remain available to reproduce earlier results; they are not additional components required by the core ZIP.

- [Model design](docs/design.md) · [Code review](docs/adversarial-review.md).
- [Funding mathematics](docs/stabilization-math.md) · [Python studies](analysis/README.md).
- [Reserve study](docs/reserve-study.md) · [Lifecycle](docs/reserve-lifecycle.md) · [Rust contract](docs/reserve-rust-contract.md) · [Reserve review](docs/reserve-adversarial-review.md).
- [Combined stress](docs/combined-stress.md).
- [Zebra block experiment](docs/zebra-development.md) · [Review](docs/development-review.md).
- [Collateral experiment](docs/collateral-profile.md) · [Review](docs/collateral-review.md).
- [Pinned sources](docs/sources.md).

Rebuild the paper with `bash scripts/build-whitepaper.sh` (Graphviz, Pandoc and LaTeX required). Original ZSV work is MIT-licensed; upstream dependencies retain their licenses.
