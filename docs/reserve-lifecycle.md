# Reserve shutdown and funding allowances

This second reserve experiment specifies terminal asset disposition and replaces lifetime trading quotas with a lifetime retained-funding budget. It preserves the first reserve study as a reproducible baseline. These are proposed research rules, not adopted funding policy.

## Proposed decisions

- Count initial reserve ZEC and every subsequent retained subsidy credit against one nonresettable allocation budget. Diverted credits do not consume it. Sales retain their proceeds but never restore allocation headroom.
- Keep gross ZEC-payout and token-sale limits per epoch, inventory bounds, freshness, a funding target and a fixed sunset. Remove lifetime gross trading stops. All stored counters remain bounded integers.
- Pause trades when the canonical ZEC observation becomes stale. Resume only if a fresh observation arrives before a fixed outage grace deadline. Close the reserve at the deadline or sunset, before user operations in that block.
- At closure, burn held tokens against recorded bad debt first. Return reserve ZEC to the NSM bucket. Sell residual tokens through a public, oracle-free descending-price auction; proceeds and subsequent scheduled funding credits go to the NSM bucket.

The cash destination preserves remaining public cash for the NSM. It can leave bad debt outstanding when reserve token inventory is insufficient. Cash-funded bad-debt purchases are an alternative policy; they need a separate settlement mechanism and participation assumptions. This experiment does not promise dollar redemption or settle all vault debts. Active principal remains unchanged, including when all outstanding tokens are held by the reserve.

## Work and verification

The lifecycle wrapper binds one immutable configuration to its state, records canonical observations, and requires block preparation before transfers or funding. It detects repeated funding at one height. The host still authenticates orders, validates observations and derives the permitted subsidy amount.

Tests cover allowance recycling, actual allocation limits, funding diversion, expiry, outage timing, auction execution, liability conservation, price sequence binding and rejection atomicity. The [implementation](../analysis/reserve_lifecycle.py), [tests](../analysis/test_reserve_lifecycle.py), [numeric cases](../analysis/lifecycle-results.json) and [Rust integration contract](reserve-rust-contract.md) are included.

Run from the repository root:

    PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s analysis -p 'test_*.py'
    PYTHONDONTWRITEBYTECODE=1 python3 -m analysis.lifecycle_study --check

## Allocation limits and continuing trade

Let $A$ be retained public ZEC allocation so far, $L$ its lifetime cap, $R$ reserve cash, $T$ the funding target and $f$ the authorized block credit. Then:

$$
r=\min(f,\max(0,T-R),L-A),\qquad A'=A+r.
$$

The reserve retains $r$; the NSM bucket receives $f-r$. Initial reserve ZEC counts in $A$. Sale proceeds never reduce $A$, and diverted credits never increase it. The acquisition cost of initial token inventory is a **separate endowment** requiring its own authorization and budget. The synthetic fixture does not prove its origin or collateral backing.

The v1 experiment stopped after 100 ZEC of gross purchases, even if earlier sales had replenished its cash. The revised policy limits retained funding while keeping gross epoch rates and a sunset. If there are $K$ active epochs and the gross payout limit is $G$ ZEC per epoch, cumulative normal-quote payouts cannot exceed $KG$. This is a different, potentially larger physical bound than the v1 lifetime gross cap. It does not bound true dollar losses.

For activation $h_a$, exclusive sunset $h_s$ and epoch length $E$:

$$
K=\left\lfloor\frac{h_s-1}{E}\right\rfloor-
\left\lfloor\frac{h_a}{E}\right\rfloor+1.
$$

After the 20-epoch sale/buyback cycle, reserve cash is 101 ZEC and retained allocation remains 100 ZEC. A later 100-token purchase succeeds: cash becomes 100.005 ZEC and cumulative gross payouts reach 100.495 ZEC. The earlier lifetime stop would reject that purchase.

Within-epoch quota occupation remains possible. This change restores future-epoch availability when assets remain; it does not establish fair access or prevent oracle-driven losses.

## Outage and closure ordering

The default maximum observation age is three blocks; the outage grace is six blocks. A report observed at height zero becomes stale at height four. Without a fresh replacement, closure occurs at height ten before user operations. A report arriving at height nine can resume trading; one arriving at height ten cannot undo the elapsed deadline.

Only an actually fresh observation resets the uninterrupted-outage clock. Review reproduced a bug where increasingly recent but still stale reports postponed closure indefinitely. The fixed rule preserves the earliest pause until genuine freshness returns. Same-sequence changes to price or observation height reject.

Configuration is embedded in the wrapper state. Every operation requires the block height to have been prepared. Funding can be credited once at each prepared height, including a zero credit. These checks strengthen the reference model; the native host must still authenticate observations/orders, derive the exact permitted funding amount and commit one complete branch transition.

Closure is irreversible on one branch. A chain reorganization must restore the appropriate prior branch state, including funding markers, outage timing and auction state.

## Closing the reserve without inventing assets

With inventory $H$, explicit bad debt $B$ and total supply $M$, closure burns:

$$
x=\min(H,B),\qquad H'=H-x,\quad B'=B-x,\quad M'=M-x.
$$

Active principal is unchanged. Hence $M=\text{active principal}+B$ remains true. Cash transfers to the NSM bucket; no ordinary holder acquires a free token allocation and no borrower receives automatic debt forgiveness.

For example, 90,000 total tokens comprise 84,000 active principal and 6,000 bad debt. The reserve owns 10,000 tokens. Closure burns 6,000, leaves supply equal to the 84,000 active principal, and puts the remaining 4,000 tokens up for sale. If inventory is less than bad debt, uncovered bad debt remains.

A first proposal distributed residual inventory to snapshot holders. Review found a profitable positioning path: with 10,000 reserve tokens and 80,000 external tokens, minting another 10,000 before the snapshot would earn roughly 1,111 extra tokens, before financing and fees. That free-distribution path was removed. Auction purchasers must transfer actual ZEC for existing tokens.

## Oracle-free inventory sale

The auction starts when normal quotes close. Its price is denominated directly in zatoshi per whole token. With elapsed blocks $t$, duration $D$, initial price $p_0$ and positive floor $p_f$:

$$
p(t)=p_f+\left\lceil
\frac{(p_0-p_f)\max(D-t,0)}{D}
\right\rceil.
$$

The defaults are 1 ZEC per token initially, a 2,000-block decline and a floor of one zatoshi per whole token. These are illustrative parameters, not a calibrated clearing rule. For $q_\mu$ micro-tokens, the buyer pays:

$$
z=\left\lceil\frac{q_\mu p(t)}{10^6}\right\rceil.
$$

Orders require available inventory, sufficient buyer ZEC, the correct nonce and a maximum-input bound. The buyer receives existing tokens, all payment goes to the NSM bucket, and active principal/bad debt/supply remain unchanged. Rounding up prevents same-price splitting from reducing payment.

If there are no bids, inventory remains available at the positive floor indefinitely. There is a specified exit path, **not guaranteed liquidation**. A missing or compromised ZEC/USD feed cannot block this sale because no conversion observation is used.

Competition, inclusion and price-step size determine value capture. At the floor, all 4,000 remaining example tokens sell for only 4,000 zatoshi. At an assumed \$100/ZEC that is **\$0.004**. The reproduced floor case is an adverse scenario, not a forecast or acceptable-value claim. Bidders or block producers could capture substantial value if competing bids are absent or censored. Removing free distributions does not make the auction economically safe.

Even competitive bidding faces discrete steps. At the same assumed ZEC price, the default quote is approximately \$1.000001 after 1,980 blocks and \$0.950001 one block later. For a token worth \$1, that first profitable step can transfer about \$499.99 on 10,000 tokens before costs. Duration and starting price therefore need calibration; a positive floor provides an execution rule rather than meaningful dollar protection.

## Recorded accounting cases

| Case | Result |
|---|---|
| Recycled trading capital | Subsequent purchase succeeds above the old lifetime gross threshold. |
| Retained-funding limit 150 ZEC | Initial 100 plus retained credits 19.9, 19.9 and 10.2 reaches exactly 150. Another 80 ZEC of the tested credits goes to NSM. |
| Closure with 6,000 bad debt | Supply and bad debt fall together; 4,000 remaining tokens stay owned by the reserve until purchased. |
| Floor-price sale | Transfers 4,000 tokens for 4,000 zatoshi without a price oracle; liabilities do not change. |
| Trickle of stale reports | Increasing sequences do not postpone closure at height ten. |

These are exact synthetic accounting cases. The first two mathematical studies remain unchanged and reproducible. The next engineering step is the signed Rust integration described in the contract; this lifecycle wrapper does not run inside the existing Rust vault ledger or Zebra adapter.
