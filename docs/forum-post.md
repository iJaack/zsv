# ZSV: a stablecoin-only alternative to general ZSAs

**Draft for community discussion. Not posted. Research prototype, not a mainnet proposal ready for activation.**

I'd like to explore a narrower question than general-purpose asset issuance on Zcash:

**Can we support one ZEC-backed, dollar-targeting stablecoin, with the vault rules enforced entirely by Zcash, without opening an arbitrary token-launch interface?**

I'm calling this experiment **ZSV: Zcash Stablecoin Vault**.

The goal is straightforward: lock native ZEC, mint a stablecoin against it, repay to recover the collateral, and allow permissionless liquidation if a vault becomes unhealthy. The collateral, debt, minting and liquidation rules would all be checked by Zcash nodes. There is no external-chain execution or custody committee.

## Why limit the scope?

I want to investigate the utility of a private dollar-denominated payment instrument without making a general token economy a prerequisite.

The proposed protocol recognizes exactly ZEC and one stablecoin. No user-defined asset IDs, issuer keys, token descriptions, NFTs, governance token or registration API. Issuance is authorized only by a valid collateralized borrowing transition.

That is a concrete consensus restriction. It isn't a claim that Zcash could prevent all memes, speculation, inscriptions or derivatives elsewhere. A fixed asset set cannot police what people build off-protocol.

## What could be reused from ZSAs?

Potentially the multi-asset conservation techniques, value commitments, note encryption, wallet-scanning work and relevant circuit gadgets. The general issuance and asset-registration surfaces would be excluded.

This would still be a new protocol profile requiring its own review. Reusing some ZSA work doesn't mean inheriting its audits unchanged or assuming the resulting upgrade would be small.

## How the native vault would work

A special vault output holds ZEC plus its owner and debt state. It can be spent only through defined transitions: deposit, borrow, repay, withdraw, close or liquidate.

The owner's signature cannot bypass debt rules. Liquidation does not need the owner's signature. Minting and debt creation happen together; burning and debt reduction happen together. Any liquidation shortfall remains visible as bad debt.

A signed price observation is accepted into Zcash state under deterministic quorum and freshness rules. Nodes use that accepted observation. The dollar price still comes from outside the chain; native enforcement does not eliminate oracle trust.

## What exists so far?

The repository contains an executable Rust research model, a whitepaper, a threat model, independent adversarial analysis and a source-pinned Zebra implementation proposal.

The model uses public vaults and balances, real signatures and integer monetary arithmetic. It is an isolated account model, not a mainnet-compatible Zebra fork. There are no live funds, deployed stablecoin or shielded stablecoin payments in this version.

The first native implementation target would keep vault health public so anyone can liquidate. Shielded payments for the fixed stablecoin would be a separate stage. Fully private vaults require a credible answer to who can construct a liquidation when the borrower disappears.

## The hard parts

The biggest unresolved issues are economic as well as technical:

- A collateralized token does not automatically stay at one dollar. The model has no unconditional holder redemption or proven peg mechanism.
- A ZEC crash can outrun collateral margins and liquidation liquidity.
- A compromised price quorum can enable excess borrowing or wrongful liquidation.
- Freezing new borrowing after any bad debt contains issuance but can also create a system-wide griefing target.
- Putting this into consensus adds risk and resource costs for ZEC users who never borrow.

Those are reasons to test the design carefully, not reasons to hide the tradeoffs behind the word decentralized.

## Feedback I'd value

1. Is a fixed stablecoin-only scope a useful alternative to general ZSAs, given its remaining financial and consensus complexity?
2. Which parts of the ZSA design could be safely reused while enforcing a closed two-asset set throughout the wire format and circuits?
3. Would a dedicated native bundle be preferable to a restricted proof/predicate mechanism?
4. What oracle, peg and loss-recovery design would justify proceeding beyond a model?
5. What evidence should a Zebra development-network implementation produce before any activation discussion?

The intent is to make the tradeoffs concrete enough to evaluate. This draft does not claim Zcash community approval, an assigned ZIP, production readiness or a launch timetable.

Technical references: [ZSA transfer/burn](https://zips.z.cash/zip-0226), [ZSA issuance](https://zips.z.cash/zip-0227), [extensible transaction format](https://zips.z.cash/zip-0248), and [private programmability research](https://forum.zcashcommunity.com/t/private-programmability-in-zcash-research-results-and-community-discussion/48016). See the repository's source map for pinned Zebra references.
