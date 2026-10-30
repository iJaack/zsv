# ZSV · Zcash Stablecoin Vault

ZSV proposes a Zcash protocol for locking ZEC and issuing one dollar-targeting stablecoin. It excludes the arbitrary asset issuance proposed by general Zcash Shielded Assets. The primary deliverable is a [Consensus ZIP candidate](zips/draft-ijaack-zsv.md), authored by Giacomo Barbieri, with public payments first and shielded payments in a follow-up. It is unnumbered and has not been submitted upstream.

The executable is a signed account-ledger model with simulated funds. Offline profiles linked to Zebra test block commitments, branch replay and serialized transparent transactions backed by fixture UTXOs. The collateral profile checks native signatures and vault spending rules; integration into Zebra's node verifier and a native stablecoin value pool remain proposed. Do not use the prototype or its fixture keys for real funds.

## Documents

- [ZIP candidate](zips/draft-ijaack-zsv.md) · [Readiness and review](docs/zip-readiness.md) · [Conformance material](zips/draft-ijaack-zsv/README.md)

- [Whitepaper](docs/whitepaper.md) · [PDF](docs/whitepaper.pdf)
- [Threat model](docs/threatmodel.md) · [Economic adversarial review](docs/economic-review.md) · [Independent code review](docs/adversarial-review.md)
- [Stabilization funding: mathematical tests](docs/stabilization-math.md) · [Reproducible study](analysis/README.md)
- [Bounded inventory reserve: implementation and adversarial results](docs/reserve-study.md)
- [Reserve shutdown and funding allowances](docs/reserve-lifecycle.md) · [Signed Rust integration](docs/reserve-rust-contract.md) · [Reserve adversarial review](docs/reserve-adversarial-review.md)
- [Combined vault and reserve stress](docs/combined-stress.md) · [Zebra-linked development](docs/zebra-development.md) · [Development review](docs/development-review.md)
- [Transparent collateral profile](docs/collateral-profile.md) · [Collateral review](docs/collateral-review.md)
- [Native Zebra implementation proposal](docs/zebra-integration.md) · [Draft ZIP](docs/draft-zip.md) · [Pinned sources](docs/sources.md)
- [Forum post draft](docs/forum-post.md) (not published)
- [Verification receipt](docs/verification.md)

## Scope

| Implemented research software | Proposed native integration | Not included |
|---|---|---|
| ZEC + one fixed stablecoin enum | Consensus vault bundle and restricted payment encoding | Arbitrary tokens, NFTs or issuer keys |
| Signed model requests, 2-of-3 oracle reports and restricted native P2PK signatures | Node transaction verification and fixed asset constraints | Bridge, external execution or custody committee |
| Borrow, repay, deposit, withdraw, close, transfer, full liquidation | Atomic branch-state validation/commit/rollback in Zebra | Mainnet node or activated upgrade |
| Checked integer math and supply/debt conservation | Wallet/library encoding and native value-pool integration | Dollar redemption, interest, general recapitalization |
| Explicit bad debt and issuance freeze | Possible fixed-two-asset shielded profile | Shielded stablecoin payments or private vaults in v0 |
| Optional signed reserve swaps, scheduled fixture funding, shutdown and inventory auction | Native reserve value path and approved coinbase/NSM accounting | Approved funding allocation or guaranteed peg |
| Zebra block parsing, commitment checks and offline branch replay | Native semantic/contextual verification and durable branch state | Mined blocks, node fork choice or crash-safe database commits |
| Branch-specific fixture UTXOs, experimental vault spend guard and per-transaction ledger reconciliation | Collateral guard in every Zebra spend and finalization path | Native zUSD value pool or reserve funding in this profile |

The fixed-asset restriction prevents arbitrary native issuance through ZSV. Off-protocol tokens, inscriptions, derivatives and speculation remain possible. The proposed cryptographic constraints and consensus code need independent review.

The ZIP candidate goes beyond the implemented model: it specifies a new native format and value pool, a 5-of-7 anchored oracle and permissionless bad-debt retirement. Existing Rust tests do not establish conformance with these proposed additions. Run `bash scripts/verify-zip.sh` for the separate candidate vectors; its oracle checker needs Python `cryptography`.

## Run locally

Requires Rust 1.90 with `rustfmt` and `clippy`; the checked-in toolchain selects the version. Cargo downloads locked dependencies on first build. The executable then runs locally without a service or network connection.

```sh
cargo test --locked
cargo run --locked --example scenario
cargo run --locked --example reserve_scenario
cargo run --locked --example combined_stress
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The scenario uses public fixture keys and simulated ZEC. It demonstrates issuance, repayment and a price crash that creates bad debt. The verification receipt records the commands, results and model limits.

The reserve example adds three signed scenarios: a reserve round trip followed by vault repayment, shutdown that covers only part of the bad debt, and an uncontested auction that sells inventory cheaply. The combined stress example varies crashes, oracle delays, liquidator funding, reserve inventory and shutdown timing through the same signed ledger. It reports recorded bad debt separately from shortfalls remaining in active vaults.

The separate funding and inventory studies need Python 3.10 or later. Run `python3 -m unittest discover -s analysis -p 'test_*.py'`, `python3 analysis/run_study.py --check`, `python3 -m analysis.reserve_study --check` and `python3 -m analysis.lifecycle_study --check` to verify their arithmetic and checked-in results. Figures additionally require Matplotlib. These are uncalibrated economic experiments; the Rust implementation checks their reserve accounting and authorization within the vault ledger.

The library accepts a `Block` through `State::validate_block` / `State::apply_block`. `Snapshot`s expose detached copies for inspection. `validate_block_with_trace` also returns snapshots after the block prelude and each transaction, once the whole block passes validation. The collateral profile uses these to reconcile native outputs after each transition. `from_regtest_fixture` preserves the original vault-only model. `from_regtest_fixture_with_reserve` enables the reserve under a separate configuration-bound chain domain. Both reject Mainnet/Testnet. Callers cannot deserialize mutable ledger state or invoke a general mint API.

The reserve starts with ZEC debited from a named fixture donor and no stablecoins. It acquires tokens issued by real model vaults through signed trades. An immutable regtest schedule supplies additional simulated ZEC once per block; retained funding has a target and lifetime cap. Swaps share gross epoch allowances. Sunset or a sustained report outage closes normal quotes, burns inventory against recorded bad debt, moves cash to an NSM accounting bucket and auctions remaining tokens. The bucket does not implement native NSM issuance.

## Design choices

- Owner-scoped vault IDs and genesis/configuration-bound signing domains protect the model against the ID squatting and cross-configuration replay reproduced during independent review. Identical canonical genesis/configuration represents the same model chain.
- Vaults and balances are public, so anyone can inspect liquidation conditions. Shielded payments are later work.
- The stablecoin's provisional name is zUSD. No deployment or ticker ownership is claimed.
- Illustrative 250% borrowing ratio, 200% liquidation threshold and 5% bonus; 100 zUSD minimum nonzero debt; 100,000 zUSD global ceiling. These are test settings, not risk recommendations.
- Liquidations burn actual stable balances, pay ZEC atomically, and retain uncovered debt. Any bad debt freezes borrowing. Reserve closure can clear only the portion covered by its existing token inventory; uncovered losses remain.
- The price quorum authenticates reports but cannot prove market accuracy or execution. Reserve freshness measures inclusion height and report expiry; the signed report has no observation timestamp. Holders have no unconditional redemption right, so the model does not guarantee a dollar peg.

## Native implementation boundary

The proposed native system gives no collateral spending or issuer key authority to override consensus rules. The account model tests those rules using simulated ZEC.

The original contextual adapter tests preparation and commit against the local model. The first [Zebra-linked runner](zebra/dev-harness) verifies a model-payload commitment in a native coinbase output and follows native parent hashes. That output commits data only; this profile has no collateral UTXOs.

The separate [collateral profile](docs/collateral-profile.md) starts from a trusted, unmined bootstrap block containing fixture funding outputs. It verifies restricted V4 P2PK spends with secp256k1 signatures, pairs every native user transaction with a signed model request and reconciles each wallet and vault against the resulting UTXOs. A separate Ed25519 intent binds the exact unsigned native transaction, model request, profile domain and parent checkpoint. The fixture explicitly binds each model owner to its distinct native key. Signatures use a fixed historical Sapling context, user transactions require finite positive expiry, and fees are zero. Reserve-enabled state and synthetic funding are rejected.

The vault marker ends in `OP_TRUE`: unmodified Zcash nodes do not protect it. Protection comes from this profile's mandatory contextual guard. Neither profile runs full Zcash consensus validation, and zUSD remains a signed model balance. The [native proposal](docs/zebra-integration.md) describes the remaining node-verifier, stablecoin value-pool, proof and durable-state work.

The Zebra-linked crate has its own pinned dependency lockfile and checks. With Cargo nextest installed, run `bash zebra/verify-dev.sh`. Initial dependency fetching needs network access; the tests then run offline without starting a node. The [development notes](docs/zebra-development.md) state which native checks remain absent.

## Rebuild the paper

With Graphviz, Pandoc and a LaTeX installation:

```sh
bash scripts/build-whitepaper.sh
```

Original ZSV work is MIT-licensed. Upstream sources retain their own licenses. No upstream code is relicensed by reference, and this repository implies no Zcash endorsement or assigned ZIP number.
