# ZSV: a stablecoin-only alternative to general ZSAs

Unpublished draft for community discussion. ZSV is a regtest research prototype; native integration and mainnet activation remain future work.

I'd like to explore one ZEC-backed, dollar-targeting stablecoin, with vault rules enforced entirely by Zcash and no arbitrary token-launch interface.

I'm calling this experiment ZSV: Zcash Stablecoin Vault.

An owner would lock native ZEC, mint a stablecoin against it and repay to recover the collateral. Anyone could liquidate an unhealthy vault. Zcash nodes would check collateral, debt, minting and liquidation rules without external-chain execution or a custody committee.

## Why limit the scope?

I want to investigate the utility of a private dollar-denominated payment instrument without making a general token economy a prerequisite.

The proposed protocol recognizes exactly ZEC and one stablecoin. It excludes user-defined asset IDs, issuer keys, token descriptions, NFTs, a governance token and a registration API. Only a valid collateralized borrowing transition authorizes issuance.

This consensus restriction cannot prevent speculation, inscriptions or derivatives built off-protocol.

## What could be reused from ZSAs?

Potentially the multi-asset conservation techniques, value commitments, note encryption, wallet-scanning work and relevant circuit gadgets. The general issuance and asset-registration surfaces would be excluded.

The resulting protocol profile would need its own review. Existing ZSA audits would not establish its security, and the upgrade could still be substantial.

## How the native vault would work

A special vault output holds ZEC plus its owner and debt state. It can be spent only through defined transitions: deposit, borrow, repay, withdraw, close or liquidate.

The owner's signature cannot bypass debt rules. Liquidation does not need the owner's signature. Minting and debt creation happen together; burning and debt reduction happen together. Any liquidation shortfall remains visible as bad debt.

A signed price observation is accepted into Zcash state under deterministic quorum and freshness rules. Nodes use that accepted observation. The dollar price still comes from outside the chain; native enforcement does not eliminate oracle trust.

## What exists so far?

The repository contains an executable Rust research model, a whitepaper, a threat model, independent adversarial analysis and a source-pinned Zebra implementation proposal.

The isolated account model uses public vaults and balances, real signatures and integer monetary arithmetic. Its optional reserve trades tokens created by model vaults, accounts for scheduled fixture funding and closes or auctions inventory under fixed rules. Native Zebra accounting, live funds and shielded stablecoin payments remain future work.

The first native implementation target would keep vault health public so anyone can liquidate. Shielded payments for the fixed stablecoin would be a separate stage. Fully private vaults require a credible answer to who can construct a liquidation when the borrower disappears.

## The hard parts

Several economic and technical problems remain unresolved:

- A collateralized token does not automatically stay at one dollar. The model has no unconditional holder redemption or proven peg mechanism.
- A ZEC crash can outrun collateral margins and liquidation liquidity.
- A compromised price quorum can enable excess borrowing or wrongful liquidation.
- Freezing new borrowing after any bad debt contains issuance but can also create a system-wide griefing target.
- Putting this into consensus adds risk and resource costs for ZEC users who never borrow.

## Could reduced grant funding support stabilization?

A separate mathematical study tests that possibility. Taking 10% of the taper discussion's rounded net reduction in future ZCG funding would imply about 6,200 ZEC over its horizon, excluding the separate treasury contribution. That allocation would reduce what otherwise goes to the NSM and needs a specified schedule and network upgrade.

Funding alone does not establish a peg. The study finds that delay narrows the controller's stable range, finite budgets leave residual deviations, and a ZEC crash weakens both collateral and dollar incentive capacity. It also finds a direct attack on raw mint/burn bounties: mint, retain the tokens, then burn them in a later rewarded epoch and collect both payments without an external trade. The current prototype has no rewards. The [math and reproducible scenarios](https://github.com/iJaack/zsv/blob/main/docs/stabilization-math.md) explain what would need to change.

The [inventory study](https://github.com/iJaack/zsv/blob/main/docs/reserve-study.md) tests standing ZEC/zUSD bid/ask exchanges with existing inventory and gross spending limits. It needs only the ZEC/USD conversion oracle. In its assumed market, enough cash and inventory support a $0.995–$1.005 corridor; purchasing 4,500 tokens costs about $4,477.50. Those Python scenarios are uncalibrated. Wrong ZEC prices, fundamental depegs and paid occupation of trading quotas remain risks.

## Shutdown and funding limits

The [lifecycle policy](https://github.com/iJaack/zsv/blob/main/docs/reserve-lifecycle.md) replaces lifetime trading stops with a cap on retained funding. Closure follows sustained report failure or sunset: existing inventory first covers recorded bad debt, cash moves to the NSM bucket, and residual tokens enter an oracle-free auction. Free terminal distributions were rejected after a borrowing-based farming counterexample. Auctions can still clear cheaply, and cash returned to NSM may leave bad debt uncovered.

These rules now run inside the [signed Rust vault ledger](https://github.com/iJaack/zsv/blob/main/docs/reserve-rust-contract.md). The reserve starts with a donor's simulated ZEC and no tokens. It can buy only tokens that vaults have issued, and all vault, reserve, funding and shutdown changes roll back if a block fails. The funding schedule is a regtest fixture, not an approved block-reward allocation. Freshness measures report inclusion age and expiry; a quorum can still publish stale market data in a new report. The next work is combined economic stress testing and an isolated native Zebra implementation.

## Feedback I'd value

1. Is a fixed stablecoin-only scope a useful alternative to general ZSAs, given its remaining financial and consensus complexity?
2. Which parts of the ZSA design could be safely reused while enforcing a closed two-asset set throughout the wire format and circuits?
3. Would a dedicated native bundle be preferable to a restricted proof/predicate mechanism?
4. What oracle, peg and loss-recovery design would justify proceeding beyond a model?
5. What evidence should a Zebra development-network implementation produce before any activation discussion?

Technical references: [ZSA transfer/burn](https://zips.z.cash/zip-0226), [ZSA issuance](https://zips.z.cash/zip-0227), [extensible transaction format](https://zips.z.cash/zip-0248), and [private programmability research](https://forum.zcashcommunity.com/t/private-programmability-in-zcash-research-results-and-community-discussion/48016). See the repository's source map for pinned Zebra references.
