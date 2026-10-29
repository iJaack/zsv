# ZSV · Zcash Stablecoin Vault

An experimental **stablecoin-only alternative to general Zcash Shielded Assets**: lock ZEC, issue one dollar-targeting stablecoin, and enforce vault transitions on Zcash.

**Research prototype. No live funds. No mainnet or public-testnet activation. No production peg or privacy claim.** The executable is a signed account-ledger model; the native Zebra implementation is proposed separately.

## Start here

- [Whitepaper](docs/whitepaper.md) · [PDF](docs/whitepaper.pdf)
- [Threat model](docs/threatmodel.md) · [Economic adversarial review](docs/economic-review.md) · [Independent code review](docs/adversarial-review.md)
- [Native Zebra implementation proposal](docs/zebra-integration.md) · [Draft ZIP](docs/draft-zip.md) · [Pinned sources](docs/sources.md)
- [Forum post draft](docs/forum-post.md) (not published)
- [Verification receipt](docs/verification.md)

## Scope

| Implemented executable model | Proposed native integration | Not included |
|---|---|---|
| ZEC + one fixed stablecoin enum | Consensus vault bundle and restricted payment encoding | Arbitrary tokens, NFTs or issuer keys |
| Real signed requests and 2-of-3 oracle reports | Native Zcash signatures/digests, fixed asset constraints | Bridge, external execution or custody committee |
| Borrow, repay, deposit, withdraw, close, transfer, full liquidation | Atomic branch-state validation/commit/rollback in Zebra | Mainnet node or activated upgrade |
| Checked integer math and supply/debt conservation | Wallet/library encoding and native value-pool integration | Dollar redemption, interest, recapitalization |
| Explicit bad debt and issuance freeze | Possible fixed-two-asset shielded profile | Shielded stablecoin payments or private vaults in v0 |

The fixed-asset restriction prevents arbitrary native issuance **through this extension**. It cannot prevent off-protocol tokens, inscriptions, derivatives or speculation. New cryptographic constraints and consensus code still need independent review.

## Run locally

Requires Rust 1.90 with `rustfmt` and `clippy`; the checked-in toolchain selects the version. No service or network connection is used by the executable. Cargo downloads locked dependencies on first build.

```sh
cargo test --locked
cargo run --locked --example scenario
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The scenario uses explicitly public fixture keys and simulated ZEC. It demonstrates healthy issuance/repayment and a price crash that creates visible bad debt. Do not use those keys for any funds. Exact commands/results and model limitations are in the verification receipt.

The library accepts a `Block` through `State::validate_block` / `State::apply_block`. Immutable `Snapshot`s expose balances and vault state for inspection. State construction is explicitly named `from_regtest_fixture` and rejects Mainnet/Testnet. There is no general mint API or deserializable mutable ledger state.

## Design choices

- Owner-scoped vault IDs and genesis/configuration-bound signing domains protect the model against the ID squatting and cross-configuration replay reproduced during independent review. Identical canonical genesis/configuration represents the same model chain.
- Public vaults and balances in the executable. Public vault health makes borrower-independent liquidation observable; shielded payments are later work.
- Single stablecoin provisional name: zUSD. The name is not a claim of ticker ownership or an existing deployment.
- Illustrative 250% borrowing ratio, 200% liquidation threshold and 5% bonus; 100 zUSD minimum nonzero debt; 100,000 zUSD global ceiling. These are test settings, not risk recommendations.
- Liquidations burn actual stable balances, pay ZEC atomically, and retain uncovered debt. A zero-tolerance bad-debt freeze contains issuance but permits griefing and has no recovery action in v0.
- The price quorum authenticates observations; it does not prove those observations represent executable market prices. There is no unconditional holder redemption, so a dollar peg is not guaranteed.

## Native implementation boundary

The proposed native system has no collateral spending or issuer key that can override consensus rules. The reference account model is useful for testing those rules, but it does **not** lock real ZEC or parse actual Zcash vault transactions.

See the source-pinned Zebra proposal for actual current APIs, required librustzcash changes, activation/digest rules, state/fork handling and acceptance gates. Any contextual adapter included here is an executable seam sketch, not a patch that activates a complete node.

## Rebuild the paper

With Graphviz, Pandoc and a LaTeX installation:

```sh
bash scripts/build-whitepaper.sh
```

Original ZSV work is MIT-licensed. Upstream sources retain their own licenses. No upstream code is relicensed by reference, and this repository implies no Zcash endorsement or assigned ZIP number.
