# Signed vault and reserve integration

The reserve and lifecycle rules now execute inside the signed Rust account ledger. [src/reserve.rs](../src/reserve.rs) implements the optional policy; [signed scenarios](../examples/reserve_scenario.rs) exercise actual vault issuance, reserve trades, liquidation and shutdown. The contextual adapter validates the combined state in model tests. Native Zebra parsing, collateral locks, proofs, coinbase rules and persistent fork state remain proposed.

## Configuration and authorization

`State::from_regtest_fixture_with_reserve` creates a reserve-enabled chain. Its immutable `ReserveConfig` defines quotes, epoch limits, funding target/budget, activation/sunset, outage timing, auction prices and a synthetic funding schedule. Initial reserve ZEC is debited from a named fixture donor and counts against the funding budget. Initial token inventory is zero. The only token source is successful vault issuance under the existing collateral and supply rules.

The new `zsv/model-chain/v2` domain hashes the legacy canonical genesis domain, every reserve configuration field in declaration order as little-endian u64, donor key and initial reserve ZEC. Transaction and oracle signatures continue to bind the chain domain under their existing v2 signing prefixes. The legacy constructor retains its exact v1 chain domain, checkpoints and signed vectors. The optional reserve snapshot is omitted when disabled.

| Action | Direction | Signed fields after its tag |
|---|---|---|
| `ReserveBuy`, tag 8 | Reserve buys the signer's existing tokens | tokens, oracle sequence, epoch, expiry, minimum ZEC output |
| `ReserveSell`, tag 9 | Reserve sells inventory to the signer | tokens, oracle sequence, epoch, expiry, maximum ZEC input |
| `ReserveAuctionBuy`, tag 10 | Signer buys residual inventory after closure | tokens, expiry, maximum ZEC input |

Each field is u64 little-endian. The surrounding transaction signs network, chain domain, signer, nonce and the ordered action list. The existing Ed25519 verifier authenticates all actions. There is no caller-authorized flag, funding action, administrator withdrawal or arbitrary asset type.

## Exact accounting

Every accepted block checks these equalities in integer units:

- Wallet ZEC + vault collateral + reserve ZEC + NSM bucket = fixture ZEC + cumulative scheduled funding.
- Wallet zUSD + reserve inventory = stable supply = active vault principal + recorded bad debt.

Funding at heights `funding_start_height <= h < funding_end_height` adds exactly `funding_zec_per_block` synthetic zatoshi per block. Configuration validation rejects schedules whose total plus fixture funds exceeds u64. Default funding is zero. Funding occurs once per consecutive accepted block; snapshots retain its cumulative amount and last processed height. Before activation and after closure, all funding goes to the NSM bucket.

While active, the retained amount is the minimum of that block's allocation, remaining cash-target headroom and remaining lifetime funding budget. The rest goes to NSM. Initial reserve cash and later retained credits consume the budget. Sale proceeds stay in the reserve and never restore budget headroom. All balances and counters use checked arithmetic. The NSM bucket records a proposed destination; it does not implement native subsidy reissuance.

For q micro-zUSD and price X micro-USD per ZEC, the bid pays `floor(q * (1e6-spread) * 1e8 / (1e6*X))` zatoshi. The ask charges the corresponding ceiling with `1e6+spread`. Zero-output trades fail. Gross epoch ZEC purchases and token sales share block-wide counters; replenishment does not refund them. Lifetime trade totals remain diagnostic counters, without the earlier lifetime trading stop.

## Oracle timing and block order

Signed reports contain a sequence, price and expiry, but no market-observation timestamp. Reserve freshness therefore means **inclusion age plus signed expiry**: an accepted quorum at height h is usable through the lesser of `h + max_oracle_age` and `valid_until`. A new sequence can carry stale market information with valid signatures. This model cannot detect that deception. Legacy vault operations still use signed expiry, without the reserve's extra inclusion-age cutoff.

Each consecutive block runs against a private copy:

1. Reset epoch counters when the height enters a new epoch. Evaluate sunset and the previous uninterrupted outage deadline before processing a replacement price.
2. If closure is due, burn `min(reserve tokens, bad debt)` once, reducing inventory, stable supply and bad debt equally. Move all reserve cash to NSM and record the reason, height, burn and transfer. Active principal and collateral stay unchanged.
3. Authenticate any oracle update. Before the deadline, a fresh accepted quorum clears a pause. At the deadline it cannot reopen normal trading. Malformed or invalid reports reject the entire block, including tentative closure.
4. Apply the deterministic funding allocation once, then execute signed vault and reserve actions in order. Closed reserves accept only inventory-auction purchases.
5. Check the combined invariants and commit the complete state, or discard it all on failure.

For a report included at height 1 with maximum age 3, no earlier expiry and grace 6, the first stale height is 5 and closure occurs at 11. A fresh report at 10 resumes quotes; a fresh report at 11 does not reopen them. Without any report, the outage starts at activation. Closure is permanent on that branch; restoring a parent restores its earlier lifecycle state.

The debt burn is one-time and precedes user transactions. A liquidation in the closing block or afterward can create new bad debt while auction inventory remains. There is no later reserve-coverage operation. Funding and auctions continue after closure under their own rules, even without an oracle.

## Terminal inventory sale

After closure, the price in zatoshi per whole token is:

`floor_price + ceil((start_price-floor_price) * max(duration-elapsed, 0) / duration)`.

The buyer pays `ceil(tokens * price / 1e6)` zatoshi. Existing tokens move to the buyer and all proceeds go to NSM. Partial fills, expiry and maximum-input checks apply; empty inventory cannot sell again. The positive floor prevents free transfers but does not ensure useful proceeds or buyers. The sale has no conversion-oracle dependency.

Returning cash to NSM can leave bad debt outstanding. It is a policy choice, not a consequence of the conservation equations. Auctions also let borrowers buy inventory and repay their own debt; cheap clearing prices can make that profitable. These risks remain under correct signatures and accounting.

## Acceptance evidence

- [Core tests](../tests/reserve.rs) cover issuance-to-repayment, retained funding headroom, activation, outage deadlines, quotes, auctions and atomic limits.
- [Independent attacks](../tests/reserve_attacks.rs) cover configuration/domain changes, signed-field mutations, epoch competition, replay, rounding, overflow, bad debt and late-block rollback.
- [Adapter tests](../tests/reserve_adapter.rs) prepare sibling branches across closure, funding and auctions, reject stale commits and reproduce a transition after restoring its parent. They do not exercise a real node reorg or disk recovery.
- [Signed vector replay](../tests/reserve_vectors.rs) checks every recorded checkpoint in three deterministic scenarios. The legacy vectors remain byte-identical.

The scenarios include a crash leaving 523.809524 zUSD of bad debt, followed by a 400-token inventory burn and an explicit 123.809524 remainder. An uncontested four-block auction sells 400 tokens for 400 zatoshi at its floor, after which the borrower repays. Its short duration deliberately illustrates a failure of competition; it is not a calibrated auction recommendation.

The [verification receipt](verification.md) records the actual test counts, commands and fingerprints. The next engineering work is economic stress testing of this combined ledger and an isolated native Zebra integration under the [native proposal](zebra-integration.md).
