# ZSV · Zcash Stablecoin Vault

ZSV proposes a Zcash protocol for locking ZEC and issuing one dollar-targeting stablecoin. It excludes the arbitrary asset issuance proposed by general Zcash Shielded Assets.

The executable is a signed account-ledger model with simulated funds. The native Zebra implementation is proposed separately. Do not use the prototype or its fixture keys for real funds.

## Documents

- [Whitepaper](docs/whitepaper.md) · [PDF](docs/whitepaper.pdf)
- [Threat model](docs/threatmodel.md) · [Economic adversarial review](docs/economic-review.md) · [Independent code review](docs/adversarial-review.md)
- [Stabilization funding: mathematical tests](docs/stabilization-math.md) · [Reproducible study](analysis/README.md)
- [Bounded inventory reserve: implementation and adversarial results](docs/reserve-study.md)
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

The fixed-asset restriction prevents arbitrary native issuance through ZSV. Off-protocol tokens, inscriptions, derivatives and speculation remain possible. The proposed cryptographic constraints and consensus code need independent review.

## Run locally

Requires Rust 1.90 with `rustfmt` and `clippy`; the checked-in toolchain selects the version. Cargo downloads locked dependencies on first build. The executable then runs locally without a service or network connection.

```sh
cargo test --locked
cargo run --locked --example scenario
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The scenario uses public fixture keys and simulated ZEC. It demonstrates issuance, repayment and a price crash that creates bad debt. The verification receipt records the commands, results and model limits.

The separate funding and inventory studies need Python 3.10 or later. Run `python3 -m unittest discover -s analysis -p 'test_*.py'`, `python3 analysis/run_study.py --check` and `python3 -m analysis.reserve_study --check` to verify their arithmetic and checked-in results. Figures additionally require Matplotlib. The inventory reserve trades existing assets without a bounty or new stablecoin-market oracle. Its gross limits bound physical spending; oracle errors and quota occupation remain risks. The studies add no incentive or reserve rules to the Rust model.

The library accepts a `Block` through `State::validate_block` / `State::apply_block`. `Snapshot`s expose copies of balances and vault state for inspection. `from_regtest_fixture` constructs the state and rejects Mainnet/Testnet. Callers cannot deserialize mutable ledger state or invoke a general mint API.

## Design choices

- Owner-scoped vault IDs and genesis/configuration-bound signing domains protect the model against the ID squatting and cross-configuration replay reproduced during independent review. Identical canonical genesis/configuration represents the same model chain.
- Vaults and balances are public, so anyone can inspect liquidation conditions. Shielded payments are later work.
- The stablecoin's provisional name is zUSD. No deployment or ticker ownership is claimed.
- Illustrative 250% borrowing ratio, 200% liquidation threshold and 5% bonus; 100 zUSD minimum nonzero debt; 100,000 zUSD global ceiling. These are test settings, not risk recommendations.
- Liquidations burn actual stable balances, pay ZEC atomically, and retain uncovered debt. A zero-tolerance bad-debt freeze contains issuance but permits griefing and has no recovery action in v0.
- The price quorum authenticates observations but cannot prove that traders can execute at those prices. Holders have no unconditional redemption right, so the model does not guarantee a dollar peg.

## Native implementation boundary

The proposed native system gives no collateral spending or issuer key authority to override consensus rules. The account model tests those rules using simulated ZEC. Native collateral locking and Zcash vault-transaction parsing still need implementation.

The Zebra proposal identifies current APIs, required librustzcash changes, activation and digest rules, fork handling, and release checks. The included contextual adapter tests preparation and commit behavior against the model. It is not linked into Zebra.

## Rebuild the paper

With Graphviz, Pandoc and a LaTeX installation:

```sh
bash scripts/build-whitepaper.sh
```

Original ZSV work is MIT-licensed. Upstream sources retain their own licenses. No upstream code is relicensed by reference, and this repository implies no Zcash endorsement or assigned ZIP number.
