# Signed reserve integration: independent adversarial review

Scope: the new reserve module, its integration into the signed Rust account ledger, and the proposed reserve lifecycle contract. This review uses simulated regtest funds and the public API. It does not review a linked Zebra implementation, consensus activation, shielded proofs or live economic parameters.

The review added 19 tests in `tests/reserve_attacks.rs`. All 19 pass. The test fixtures issue stablecoins through signed collateralized vault actions before transferring tokens to the reserve. No fixture injects token inventory or synthetic aggregate debt.

## Finding fixed

**Low: the detached auction quote helper could panic on malformed public snapshot data.** `ReserveState::auction_price` previously subtracted the configured floor from the start price before validating the publicly mutable detached configuration. A caller could set floor greater than start and trigger an unsigned-subtraction panic. This was not a ledger mutation or consensus attack: the authoritative state is private and immutable configuration is validated at construction. The defect was identified by source inspection; a pre-fix executable reproduction was not run. The implementation now validates the configuration at the helper entry. The issue was identified by source inspection; no pre-fix failing execution was recorded. The regression checks both an inverted range and a zero duration; both return errors, while the authoritative state's quote remains unchanged. Targeted Clippy also passes with warnings denied.

No remaining exploitable accounting or authorization defect was found in the inspected paths. This is a bounded review, not a security audit or economic proof.

## Tested attack paths

- Every reserve configuration field and donor/seed selection changes the chain domain. Transactions and oracle reports from another configuration are rejected.
- Mutating any field of each of the three new order variants invalidates its signature. Users cannot submit funding or arbitrary-asset operations, and detached snapshots cannot change authoritative balances.
- Expired orders, mismatched oracle sequences or epochs, wrong nonces and duplicate same-block transactions fail atomically.
- Competing fills share gross epoch allowances. Round trips retain sale proceeds without refunding gross usage or the cumulative retained-funding allowance.
- A later failed action restores a preceding valid reserve fill and the block's funding. A failed closing block restores the bad-debt burn, reserve cash, supply, funding and closure receipt.
- Liquidation-created bad debt is covered only by actual reserve inventory. With 1,000 tokens of inventory and 761.904762 tokens of bad debt, closure burns exactly 761.904762. With 200 tokens, 561.904762 remains and borrowing stays frozen.
- The previous outage deadline takes precedence over a newly included report. Signed expiry can start the outage before the inclusion-age limit. Old sequences cannot refresh inclusion time.
- Auction fills work with an expired stored oracle. They enforce price, expiry, inventory and nonce bounds, move ZEC only to the NSM bucket, preserve active debt, and round tiny fills up to a positive payment.
- Normal quotes remain closed despite new oracle reports, funding and auction proceeds. Extreme orders fail without changing state. A funding schedule whose known total would overflow ZEC accounting is rejected at construction.

## Policy limitations confirmed

**The terminal burn is one-time and precedes user transactions.** A liquidation in the closing block can create new bad debt after the burn while residual reserve tokens still exist. A later block does not repeat the burn. The regression demonstrates 761.904762 tokens of new bad debt with 1,000 reserve tokens still offered in the auction. This matches the specified ordering; it is not full-system debt settlement.

**Inclusion freshness does not prove market freshness.** The signed oracle format contains sequence, price and expiry, but no market observation timestamp. An authorized quorum can re-sign an old price under a higher sequence and reset the reserve inclusion clock. Reserve orders use the inclusion-age limit and signed expiry; ordinary vault operations preserve their existing signed-expiry rule. A paused reserve therefore does not globally pause vault issuance.

**Accounting does not establish the peg or recovery value.** Normal reserve purchases can continue while bad debt exists. Fixed quotes can spend ZEC on impaired tokens. Returning cash to the NSM can leave losses, and an auction may sell remaining inventory cheaply. Gross epoch quotas can still be occupied by economically losing trades. The retained-funding allowance bounds retained public allocations rather than guaranteeing solvency.

**Native enforcement remains future work.** The schedule is immutable synthetic regtest funding, and the NSM is an accounting bucket. Real coinbase funding, native asset/value-pool enforcement, persistent branch state, parser limits and shielded proofs remain outside this implementation.

**Resource controls remain incomplete.** The default 1,024-account cap bounds accepted account count but a payer can occupy slots using many keys, denying new users. Used vault identifiers remain permanently recorded and can grow over successive blocks. The account cap is not a complete native state-growth policy.

## Strengths

The combined ledger verifies real signatures, requires actual vault issuance before reserve token acquisition, conserves wallet/vault/reserve assets atomically, binds the full policy and funding schedule into transaction authority, and separates terminal inventory sales from debt forgiveness. Checked integer calculations and the positive auction floor prevent dust fills from transferring inventory for zero payment.

## Reproduction and reviewed source

Run `cargo test --locked --test reserve_attacks` and `cargo clippy --locked --test reserve_attacks -- -D warnings`. The main agent also ran the complete [verification script](../scripts/verify.sh); its results are in the [release receipt](verification.md).

Reviewed SHA-256 fingerprints after the helper fix:

- `src/lib.rs`: `efa3702d818a0453863b6e59bc7a8d11393c1802c9c5ae86541a10c303a9ffa6`.
- `src/reserve.rs`: `a219be80b71985a984b84eb626057908a001b538914d9bb9893fe09bd6c07c81`.
- `tests/reserve_attacks.rs`: `7a16cff322c82f34fecdce60acfc64a425ed4152ebe68382114a6d6006ca9208`.

The review did not include fuzzing, exhaustive state-space exploration, a dependency audit or a native node build.
