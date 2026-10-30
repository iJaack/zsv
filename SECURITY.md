# Security status

Do not send funds to ZSV or use its public fixture keys. The model uses simulated ZEC, and no Zcash mainnet or public-testnet activation exists. Its tests check model behavior; consensus compatibility, economic solvency, privacy and peg stability require separate validation.

Read `docs/threatmodel.md`, `docs/economic-review.md`, `docs/adversarial-review.md` and `docs/zebra-integration.md` before extending the model or implementing native support.

Reserve funding and the NSM bucket are simulated ledger entries under immutable regtest parameters. Shutdown can cover only recorded bad debt backed by the reserve's actual token inventory. Signed reports authenticate a quorum's price and expiry; reserve inclusion-age checks do not prove when that price was observed in a market.

Report reproducible model issues in this repository without secrets or real user data. Before any deployment with real funds, establish a security contact and coordinated disclosure policy.
