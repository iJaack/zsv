# Security status

Do not send funds to ZSV or use its public fixture keys. The model uses simulated ZEC, and no Zcash mainnet or public-testnet activation exists. Its tests check model behavior; consensus compatibility, economic solvency, privacy and peg stability require separate validation.

Read `docs/threatmodel.md`, `docs/economic-review.md`, `docs/adversarial-review.md`, `docs/development-review.md`, `docs/collateral-profile.md`, `docs/collateral-review.md` and `docs/zebra-integration.md` before extending the model or implementing native support. The original Zebra-linked runner binds data only. The separate collateral profile checks fixture UTXOs and vault spending rules offline; neither profile activates node consensus or creates a native stablecoin value pool.

The collateral profile's vault script is anyone-can-spend under unmodified Zcash rules. Its safety depends on a contextual guard that checks every native input against its branch UTXO set and a paired, authorized model transition. Restricted P2PK ownership uses real secp256k1 signatures; a separate Ed25519 intent binds the native transaction and model request to the profile and parent checkpoint. The trusted bootstrap supplies simulated funding without subsidy, maturity or proof-of-work validation. The profile rejects reserve-enabled genesis, synthetic funding, shielded transfers and unsupported transaction formats.

Reserve funding and the NSM bucket are simulated ledger entries under immutable regtest parameters. Shutdown can cover only recorded bad debt backed by the reserve's actual token inventory. Signed reports authenticate a quorum's price and expiry; reserve inclusion-age checks do not prove when that price was observed in a market.

Report reproducible model issues in this repository without secrets or real user data. Before any deployment with real funds, establish a security contact and coordinated disclosure policy.
