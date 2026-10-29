# Economic and protocol review

This is an adversarial review of a research prototype: one native USD-denominated asset, ZEC-only collateral, public vaults, a transparent stablecoin ledger, a canonical on-chain oracle, owner repayment and permissionless atomic liquidation. It evaluates rules that a future consensus proposal could implement. It is not evidence that Zcash currently supports these transitions, that the asset can maintain a dollar peg, or that any parameters are safe for real funds.

The review assumes the working prototype uses a 250% minimum collateral ratio for new debt and collateral withdrawal, a 200% liquidation threshold, a 5% liquidation bonus, no interest or fees, a 100-unit minimum nonzero vault debt, and a 100,000-unit system debt ceiling. These are conservative-looking simulation inputs, not calibrated risk limits. A larger collateral ratio buys time during some price declines; it does not create a guaranteed price floor.

## Correct accounting is necessary and insufficient

Use integer arithmetic throughout. Let `q = 100,000,000` zatoshi per ZEC, `P` be integer micro-USD per ZEC, `C` a vault's zatoshi collateral, and `D` its integer micro-unit debt. For the current 5% bonus, use the exact rational `n / d = 105 / 100`. The protocol must define and bound each intermediate multiplication; a correct mathematical expression can still overflow a machine integer.

With zero fees and no independent stablecoin issuance, the central monetary invariant is:

```text
stablecoin total supply = sum(active vault debt) + explicit bad debt
```

All stablecoin balances and any future shielded-pool balance count toward supply. Lost keys do not extinguish debt. A transfer cannot change supply. Minting `m` creates exactly `m` debt and exactly `m` units. Owner repayment burns exactly the debt reduction. Ordinary voluntary burns need an explicit policy: burning units without reducing a named liability breaks this equality unless a separate surplus/burn accounting term is introduced. That added term is unnecessary for this prototype.

For a vault that has become unsafe, prefer a full-close operation with the following exact calculations:

```text
x = min(D, floor(C * P * d / (q * n)))
z = ceil(x * q * n / (P * d))
burn x liquidator units
transfer z zatoshi to the liquidator
return C - z zatoshi to the vault owner
close the vault
increase bad debt by D - x
```

Require a positive, valid `P`. For these formulas, `z <= C` follows from the affordability calculation. Treat a violation as an arithmetic or validation bug; silently capping an incorrectly calculated seizure can hide overcharging. Rounding the affordable burn upward instead would ask the liquidator to overpay by up to one micro-unit and is unnecessary. A zero numerator must produce zero in ceiling division, without subtraction underflow.

When `x < D`, the returned collateral is worth less than 1.05 micro-USD at this oracle price. This is an explicit permitted rounding residual, not meaningful recapitalization. A whole-vault model can return that residual and preserve the supply identity. Do not quietly forgive the remaining debt: moving `D - x` to bad debt is what preserves it.

At sufficiently low prices, `x = 0` while `C > 0`. A zero-burn close is arithmetically coherent: no collateral is seized, the bounded dust is returned, and the entire debt becomes bad debt. It has no financial incentive for an external caller. Rejecting this case instead leaves an insolvent vault open. Either choice needs an explicit rule and a liveness limitation; neither recovers the missing dollars.

An explicit bad-debt balance measures losses. It does not fund them. Burning existing stablecoin against bad debt reduces both supply and bad debt by the same amount, but no holder is compelled to donate. Repayment of a healthy owner's own loan cannot also be counted as covering bad debt.

## The dollar peg is a market hypothesis

Debt is denominated in dollars, but this design gives a generic holder no unconditional right to exchange one unit for one dollar or one dollar of ZEC. Only vault owners can recover their own collateral through ordinary repayment; liquidation becomes available only for unsafe vaults. The documents should call this a soft peg and avoid unqualified claims of dollar redemption.

When the token trades at $0.80, a borrower who owes 100 can spend $80 buying 100 units, burn them, and recover the vault collateral. That contraction can support demand, but it requires borrowers with capital, exchange access and reason to close. Someone who holds the token but owns no vault cannot force this trade. Below-peg prices can persist when borrowers are inactive, collateral is inaccessible, or the system has bad debt.

When the token trades at $1.20, a new borrower may lock $250 of ZEC, mint 100 and sell for $120. That can increase supply, but capital costs, ZEC exposure, transaction fees, the debt ceiling, oracle pauses and borrowing freezes limit the response. Zero protocol fees do not eliminate those costs. A system-wide mint freeze can preserve remaining collateral while leaving the market without an above-peg expansion mechanism.

No native governance token or arbitrary asset issuance is needed for this model. That simplification also removes a possible recapitalization mechanism. If debt exceeds recoverable collateral, a proposal must identify who absorbs the loss: holders through a depeg or haircut, voluntary contributors, a prefunded reserve, or some explicitly specified future mechanism. An unspecified bailout is not a mechanism.

## Liquidation can fail while every transition is valid

At an oracle price of $100/ZEC, a vault with 2.5 ZEC and 100 debt opens at 250%. If ZEC gaps to $36 before a liquidation can execute, its collateral is worth $90. With the 5% bonus, a full close burns approximately 85.714285 units and moves approximately 14.285715 units into bad debt. The remaining stablecoin supply still exists and no longer has collateral in this vault. The debt ceiling limits the nominal size of this loss, not its percentage.

Liquidators need stablecoin that already exists. The protocol cannot assume they can buy it, borrow it or flash-borrow it at any time. A small global supply, concentrated holdings or exchange disruption can prevent liquidation even when collateral is valuable. An atomic burn-and-seize operation removes settlement risk between those two legs; it does not create liquidity or guarantee transaction inclusion.

For a liquidator burning `x` units, ignoring rounding, the received collateral has market value:

```text
x * 1.05 * (market ZEC price / oracle ZEC price)
```

The trade is attractive only when that value exceeds the market cost of `x` stablecoins plus execution, hedging and inclusion costs. With an oracle still at $100 but market ZEC at $40, a supposed 5% bonus delivers only $0.42 of collateral per unit burned. Rational liquidators will not buy a $1 token for that trade. A larger configured bonus cannot solve an unbounded stale-price error.

Full close is a useful prototype simplification, but it requires a liquidator to fund the whole affordable debt. It may force a large sale and realize a loss that gradual liquidation could have avoided. Partial liquidation is not a trivial extension. If collateral value is `V`, debt is `D`, and repayment is `x`, the remaining ratio becomes:

```text
(V - 1.05*x) / (D - x)
```

It improves only when the starting ratio is greater than 105%. Below that point, a partial liquidation with the full bonus worsens the residual position. A future partial-liquidation design needs an exact target ratio, minimum action size, rounding rules, residual-debt floor and explicit insolvency handling; repeating small operations must not produce a different loss allocation through rounding.

Liquidation at less than 200% must be defined using an exact cross-product comparison. Whether equality is safe or liquidatable is a consensus choice. Collateral withdrawals must check the post-withdrawal ratio at the stricter opening requirement. An owner must not be able to use a separate withdrawal operation to bypass minting constraints.

## Canonical oracle does not mean trustworthy oracle

Putting an accepted price on-chain ensures that nodes use the same price for a given branch and state. It does not make that price true. Reporter keys, their quorum, data-source concentration, freshness policy and update inclusion are part of the asset's trust model.

A concrete inflation attack is a fabricated price of $1,000 when ZEC is actually $100. At a 250% opening ratio, one ZEC can now support 400 debt even though its real collateral value is only $100. Restoring the correct price cannot undo the spendable units already minted. A debt ceiling bounds the number of units created through this path; it does not protect existing holders from dilution of backing. A reporter bond helps only if it is enforceably available and large enough to cover the loss, with additional care if that bond is also ZEC.

A fabricated low price harms borrowers. A vault with 4.75 ZEC and 100 debt is actually worth $475 at $100/ZEC. A false $40 price marks it at 190%, allowing liquidation. Burning 100 units then seizes 2.625 ZEC, worth $262.50 in the real market, while the owner receives the remainder. Deterministic validation accepts this theft if the oracle update was authorized. Calling reporters merely a data service understates their authority.

Distinct signatures are not necessarily independent observations: multiple reporters may share one exchange, administrator, cloud provider or compromised dependency. Duplicate reporter identities must not count twice. Quorum agreement must bind the same price, epoch, network and ruleset, with deterministic ordering and replay protection. A median is useful only when its honest-majority and data-quality assumptions actually hold.

Freshness and emergency policies create unavoidable tradeoffs:

* Allowing stale-price borrowing or withdrawals can leak collateral or create unbacked debt. Pause these risk-increasing operations on stale or missing prices.
* Allowing stale-price liquidation can seize sound collateral at a false low price or fail to attract liquidators at a false high price.
* Pausing liquidation on stale prices avoids that seizure path but lets market losses accumulate during an oracle outage.
* Allowing repayment, collateral additions and debt-free closure can remain safe during an oracle outage when they do not depend on a new valuation.

Consensus must use committed chain state, not a validator's wall clock, private exchange query or locally chosen price cache. Block-height freshness is deterministic but is not a wall-clock guarantee when blocks slow or halt. Reorganizations must roll back price state, debt, collateral and stablecoin supply together. A block producer's ability to delay a price update or order a liquidation before it is a real extraction and censorship surface.

## Bad-debt freeze is containment with an availability cost

Disabling new borrowing whenever explicit bad debt is positive is a clear conservative prototype rule. Keep repayment, adding collateral, bad-debt coverage and permitted liquidation available. Define the debt ceiling against all liabilities, including bad debt, rather than making liability disappear when a vault closes.

The strict freeze also means a one-micro-unit shortfall can stop borrowing for the entire system. A vault with 100 debt and collateral worth just below $105 cannot fund a full 100-unit burn with the bonus. One micro-unit of bad debt is enough to trigger the freeze. Debt floors prevent initially tiny loans, but not tiny residual shortfalls after a crash. An attacker does not need a separate token-creation API to exploit a cheap opportunity to leave this residual. A reserve or different production freeze policy could address availability; silently rounding the loss away would falsify the accounting.

The minimum debt is an entry barrier and a dust-control measure, not a defense against a large crash. The system ceiling is a nominal exposure limit, not proof that ZEC markets can absorb liquidation sales. Liquidators selling seized ZEC may depress ZEC further, make more vaults unsafe and deepen the shortfall. Stress work must model market depth and token availability as well as price paths.

## One native asset narrows issuance; it cannot prohibit memecoins

A consensus rule with one fixed asset identifier and no arbitrary issuer, ticker or metadata constructor can prevent creation of additional native assets through this feature. It reduces parser, accounting, wallet and asset-discovery complexity compared with a general multi-asset interface. Mint authorization should always come from valid debt creation; neither an owner signature nor an oracle signature alone should mint units.

The rule cannot prevent external IOUs, custodial tokens, inscriptions, wrapped claims, sidechains, contracts elsewhere, off-chain ledgers or applications that label ZEC outputs as other assets. Nor can it prevent speculative trading of the stablecoin itself. “One native asset” is an enforceable scope constraint; “no memecoins on Zcash” is not a credible consequence.

All units should have the same protocol rights regardless of which vault created them. Yet a transparent ledger reveals provenance and enables counterparties to discriminate by history. Protocol fungibility does not guarantee market fungibility. Public vaults also expose borrowers' collateral, leverage and liquidation points. A later shielded payment subset could hide transfers while keeping issuance and liabilities auditable, but it adds separate supply-conservation, proof, viewing and wallet problems. It should not be assumed to inherit current Zcash shielded support automatically.

## Concrete acceptance and stress scenarios

The first group checks prototype correctness; the second tests economic assumptions. Passing the first group is not evidence that the second is safe.

| Scenario | Required observation |
| --- | --- |
| Mint, transfer, partial repayment, full repayment | Supply changes exactly with debt changes; transfers do not change either; collateral returns only through authorized valid operations. |
| Full liquidation with sufficient collateral | Exact full debt burn, rounded-up bonus seizure no greater than locked collateral, owner remainder, no new bad debt. |
| Insolvent full liquidation | Exact affordable burn, entire old vault removed, shortfall moved to bad debt; `supply = active debt + bad debt` still holds. |
| Price and amounts around every floor/ceiling boundary | No overflow, underflow, negative balance or extra micro-unit; use adjacent integer values around 250%, 200% and 105%. |
| Positive collateral with affordable burn zero | Chosen dust-close or pending-vault behavior is explicit, deterministic and consistent with liability accounting. |
| Exact threshold equality | Mint/withdraw and liquidation comparators agree with the published specification; no branch-dependent acceptance. |
| Missing, stale, zero, duplicate, replayed or unauthorized price data | Invalid updates fail deterministically; safe owner operations remain available under the stated outage rules. |
| Reorganization spanning price update, mint and liquidation | All state rolls back together; no orphan-derived price, duplicated collateral or surviving minted balance. |
| One-micro-unit bad debt, then exact coverage | Borrowing freezes and resumes exactly as specified; coverage burns existing supply once. |
| ZEC gaps from $100 to $36 before inclusion | Record the approximately 14.285715 shortfall on the example vault; do not mark successful liquidation as successful solvency. |
| Market price $40, oracle $100 | No assumed liquidation at the advertised 5% bonus; quantify delay and resulting losses. |
| Fabricated high and low oracle reports | Measure mint-driven insolvency and borrower value extraction despite valid quorum signatures. |
| Stablecoin at $0.80 and $1.20 | Model who can actually arbitrage, how much collateral and liquidity they need, and how caps or freezes stop the trade. |
| Most stablecoin held by inactive or illiquid accounts | Measure how much debt can be liquidated with the units actually available to liquidators. |
| Block producer delays oracle updates and liquidation | Quantify losses and ordering profit; deterministic transitions alone do not resolve the attack. |
| Simultaneous vault liquidation into shallow ZEC markets | Apply sale slippage and feedback into subsequent collateral prices, rather than valuing every seizure at one unchanged price. |

Before proposing production parameters, add independently reviewed market and oracle data, explicit acceptable-loss targets, a defined loss-allocation mechanism, liquidity stress results, consensus and cryptographic review, and an operational failure plan. This prototype can establish that the accounting and state transitions are coherent. It cannot establish that a dollar claim backed only by volatile ZEC remains economically solvent under arbitrary conditions.
