# Next milestone: signed vault and reserve integration

This contract is specified against the existing Rust account ledger. It is not an implemented Rust reserve or patched Zebra node. The [lifecycle experiment](reserve-lifecycle.md) supplies the accounting and state-transition reference.

## State and authorization

Introduce an explicitly versioned experimental configuration containing reserve inventory, fixed bid/ask parameters, gross epoch rates, retained-funding budget/target, activation/sunset, outage grace and terminal auction parameters. Bind this configuration to the model chain domain. Preserve the current v1 vectors as their own versioned baseline.

Add reserve balances to the same supply/ZEC invariants as wallets and vault collateral. Include explicit retained-funding totals, once-per-block credit markers, canonical observation content, outage start, closure receipt and auction start height in snapshots and branch checkpoints. Funding counts and the NSM bucket are part of exact ZEC conservation.

Signed reserve buy/sell actions bind direction, amount, nonce, observation sequence, epoch, expiry and minimum-output/maximum-input limits. A wind-down purchase binds amount, nonce, expiry and maximum ZEC input; its price derives from the candidate block height and immutable auction parameters. Reuse the actual signature verifier. Names or caller-supplied authorization flags must never substitute for signatures.

## Block transition order

1. Resolve the exact parent model state and next block height. Evaluate whether the prior outage deadline or sunset has already elapsed. At an elapsed deadline, normal trading closes before a late observation could rescue it.
2. Authenticate the block's canonical observations under the specified ordering. Preserve one price/height per sequence. Before the closure deadline, a fresh observation may clear an outage. Define report-observation time separately from inclusion time; the existing report format needs an explicit decision here.
3. Apply deterministic closure once when required. Burn reserve tokens and reduce explicit bad debt by the same amount. Transfer cash to the NSM allocation path and retain residual inventory for auction. Active vault principal and collateral remain unchanged.
4. Derive the authorized block-funding amount from the chosen regtest schedule. Credit it once, retaining only available budget/target headroom. After closure, route the entire allocation to the NSM path. Users cannot submit funding credits.
5. Execute signed vault/reserve operations in canonical order against a private overlay. Same-block fills share counters and balances. Validate all ordinary and reserve invariants before commit.
6. Commit or reject the whole block atomically. Fork/replay restores counters, credits, oracle state, bad-debt burn, auction state and wallets together.

The Python prototype relies on host-supplied funding amounts and authenticated inputs; the Rust milestone must close those gaps. Native coinbase funding and NSM rules require the separately approved network schedule rather than the regtest fixture schedule.

## Required acceptance cases

- A valid vault issues tokens, sells some to the reserve, later buys inventory and repays. Every step preserves the combined supply/debt and ZEC invariants.
- Liquidation produces bad debt; closure burns only actual reserve inventory against that bad debt. Partial coverage leaves an explicit remaining loss.
- A late failure after a reserve fill, funding credit or closure rolls back the complete block.
- Same-block competing fills cannot exceed the epoch allowance. A later epoch can use replenished capital without refunding the retained-funding budget.
- Duplicate block credits, cross-configuration orders, stale quote sequences and wrong nonces fail.
- Always-stale reports cannot restart the outage clock; fresh reports before and at the deadline have the specified different outcomes.
- Auction fills work after oracle failure, conserve assets, respect signed input bounds and reject replays. Empty inventory cannot sell again.
- Sibling branch preparation and reorg replay restore the exact state across expiry and auction fills.

## Limits that remain economic

Returning cash to the NSM can leave uncovered bad debt. Per-epoch capacity can still be occupied by losing cycles. Auctions can clear cheaply under poor participation or censorship. Neither integer accounting nor signature checks removes those choices and risks.

Only after this combined Rust milestone passes review should the reserve be connected to an isolated Zebra development network. The native parser/proof/value-pool, persistent storage and fork requirements remain those in the [Zebra proposal](zebra-integration.md).
