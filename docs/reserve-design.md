# Bounded ZEC / zUSD reserve experiment

This document records the original Python reserve design. The [signed Rust integration](reserve-rust-contract.md) uses its later lifecycle rules, including a retained-funding budget instead of lifetime trading stops.

This experiment replaces activity bounties with direct, atomic exchanges of existing ZEC and zUSD. The user authorized building and testing this mechanism after the first study identified reward farming. Implementation choices below are research assumptions.

## Design choice

The reserve continuously offers a target-centred bid and ask, using the accepted ZEC/USD price for conversion. Participants choose when to fill the quotes. All purchases retain token inventory; all sales require existing inventory. Neither operation mints tokens, burns debt or releases vault collateral.

Raw mint/burn bounties pay for observable activity without establishing market impact. A constant-product reserve would price ZEC against zUSD through its inventory curve but would still need a dollar reference and an explicit intervention policy. Fixed bid/ask exchanges make inventory, rounding and spending bounds easier to inspect for this experiment.

Illustrative quotes are a $0.995 bid and $1.005 ask. These are test settings. The reserve requires a fresh ZEC/USD observation, order price limits and expiry; it enforces gross outflow caps per epoch and over its lifetime, an inventory cap, a ZEC floor and a sunset. A funding target diverts excess new subsidy credits to the NSM. Sales retain their proceeds so later purchases can use that working capital. The design needs no additional stablecoin-market oracle. True market prices and dollar losses are measured separately in the study; there is no hard dollar-loss guarantee.

## Accounting and scope

The exact exchange ledger uses integer zatoshi and micro-zUSD, matching the existing Rust units. Every exchange conserves each asset across the reserve and participant. Stablecoin supply, active debt and bad debt remain unchanged. Initial reserve tokens must transfer from a named funded fixture account and already count toward supply and debt. Subsidy funding is a separately recorded hypothetical ZEC credit; excess above the funding target is recorded for the NSM. Retained sale proceeds can put cash above that target without creating a new subsidy allocation.

The Python core tests accounting and policy enforcement. It assumes the host supplies a canonical unchanged policy and exact branch state, authorizes the actor, authenticates observations and ensures each sequence denotes one price/height. The host also derives funding amount and once-per-block cadence; the funding helper does not enforce an emission schedule. The core supplies no new cryptographic authorization, native transaction parser, proof or Zebra execution path. The separate market simulation uses assumed executable prices and participation; balances alone do not prove circulation or market impact.

## Implementation and review plan

1. Write accounting, rounding, replay, cap and adverse-input tests before implementing the integer exchange core.
2. Run finite-inventory market scenarios and independently value hostile trades at assumed true prices.
3. Derive inventory feasibility, intervention capital and fixed-price round-trip bounds. Test stale prices, compromised prices, persistent depegs, transaction hurdles, funding overflow and expiry.
4. Have separate reviewers inspect the implementation and adversarial economics. Fix reproducible findings.
5. Update the mathematical report, whitepaper, threat model, forum draft and native Zebra proposal. Run complete verification and reproduce numeric outputs before the authorized public push.

## Questions this experiment must answer

- How much ZEC and initial token inventory does intervention actually need?
- Which conditions stop both purchases and sales, and which merely limit losses over time?
- Does repeated trading or resetting an epoch bypass a spending limit?
- Which round trips are loss-making at unchanged prices, and which oracle errors permit extraction?
- What happens when demand exceeds all existing supply or the token loses fundamental value?

The experiment cannot establish market calibration, an enforceable dollar redemption right or a safe production oracle. Those remain prerequisites for any value-bearing deployment.
