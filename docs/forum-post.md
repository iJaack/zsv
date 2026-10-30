# ZSV: one ZEC-backed stablecoin on Zcash

Unpublished discussion draft by Giacomo Barbieri.

I'd like feedback on [ZSV](https://github.com/iJaack/zsv/blob/main/zips/draft-ijaack-zsv.md), a proposed Consensus ZIP for borrowing one dollar-targeting stablecoin against ZEC. Zcash nodes would enforce the vault and payment rules. The draft is unnumbered and has not been submitted upstream.

## What the first version does

- Locks ZEC in a dedicated vault value pool.
- Issues zUSD when an owner borrows against sufficient collateral.
- Burns existing zUSD on repayment or liquidation, accounting for any uncovered debt.
- Allows public zUSD payments and permissionless liquidation.
- Accepts ZEC/USD reports signed by five of seven upgrade-selected reporters, with an observation anchored to a recent Zcash block.

Vaults and payments are public. Shielded payments would need a follow-up ZIP. There is no arbitrary asset registration, issuer key, governance token or general execution environment. This restriction cannot prevent off-protocol derivatives or speculation.

The draft uses a 250% borrowing ratio, liquidation below 200%, a 5% liquidation bonus and a 100,000-zUSD supply cap. These are initial research parameters requiring calibration.

## The difficult tradeoffs

A dollar target does not guarantee a dollar price. There is no unconditional redemption right. A ZEC crash, false oracle price or shortage of liquidators can leave losses even when every node follows the rules correctly.

Recorded bad debt stops new borrowing. Holders may burn existing tokens without compensation to retire that debt; borrowing resumes only after it reaches zero. If nobody accepts that loss, the freeze can last indefinitely. This recovery policy and the fixed reporter committee need scrutiny.

Public vaults expose financial positions. Public payments expose amounts and transaction relationships. Adding shielding later would not erase those disclosures.

## Why keep it narrow?

The immediate question is whether one native stablecoin justifies the additional consensus and oracle risk. Public v1 needs no new shielded circuit or general ZSA issuance machinery. Selected ZSA transfer techniques may be useful for later shielding.

Dev Fund incentives and a stabilization reserve are separate research. The [funding study](https://github.com/iJaack/zsv/blob/main/docs/stabilization-math.md) and [reserve results](https://github.com/iJaack/zsv/blob/main/docs/combined-stress.md) show finite-budget, stale-price and shutdown risks. No funding allocation is assumed by this ZIP.

## What exists

The repository contains the draft, a whitepaper, a threat model, financial and encoding fixtures, an executable account model and offline Zebra-linked experiments. These use simulated funds. The experiments do not implement the candidate's native format or stablecoin value pool, and their reviews do not replace qualified independent review.

The [implementation plan](https://github.com/iJaack/zsv/blob/main/docs/implementation-plan.md) follows the public candidate directly: transaction-library support, consensus verification and durable branch state.

## Feedback requested

1. Does this limited use case justify a Zcash consensus change?
2. Are the oracle committee and uncompensated loss-retirement policy acceptable? What concrete alternative would improve them?
3. Are the transaction and value-pool rules practical for Zcash libraries and Zebra?
4. What economic evidence would justify the parameters and dollar target?
