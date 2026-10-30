# Stabilization funding study

This study tests a proposed use of a fraction of future ZCG funding reductions. It adds no rewards, oracle or monetary rules to the Rust model.

The work has four parts:

1. Exact rational and integer funding splits, including the distinction between a renewed 8% reference and the existing funding expiry.
2. Closed-form local stability and reserve-cost bounds for an assumed supply response.
3. Deterministic finite-budget scenarios with collateral and debt limits.
4. Adversarial counterexamples for rewards, price observations and wallet caps.

The funding and controller tests were written before their implementation. A separate reviewer checked the equations and executable study. Results are conditional model calculations; no parameter is calibrated to ZEC markets or borrower behavior.

Run the standard-library checks and regenerate numeric outputs:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s analysis -p 'test_*.py'
PYTHONDONTWRITEBYTECODE=1 python3 analysis/run_study.py
PYTHONDONTWRITEBYTECODE=1 python3 analysis/run_study.py --check
```

Generate the figures with Matplotlib 3.10.8 (the numeric study has no third-party dependencies):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 analysis/run_study.py --plots
```

Read [the mathematical report](../docs/stabilization-math.md). `results.json` contains assumptions, funding calculations, attack examples and scenario summaries. `trajectories.csv` records every simulated epoch. The charts contain selected scenarios, not forecasts or probability estimates.

## Inventory reserve

The next experiment replaces activity rewards with standing ZEC/zUSD bid/ask exchanges. [Its report](../docs/reserve-study.md) covers integer transfers, inventory, gross caps, oracle attacks and quota occupation. The exact ledger assumes host-authorized orders, canonical policy and price observations, and schedule-authorized funding; it supplies no signature verifier or native transaction adapter.

From the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m analysis.reserve_study
PYTHONDONTWRITEBYTECODE=1 python3 -m analysis.reserve_study --check
PYTHONDONTWRITEBYTECODE=1 python3 -m analysis.reserve_study --plots
```

The final command additionally requires Matplotlib. `reserve-results.json`, `reserve-trajectories.csv` and `reserve-sensitivity.csv` contain 11 scenarios and 144 parameter combinations. The same unittest command above runs both studies' tests.

## Lifecycle and allocation limits

The [lifecycle extension](../docs/reserve-lifecycle.md) embeds the policy, caps retained new funding, preserves epoch trading limits, pauses on stale observations and closes at an outage deadline or sunset. Closure burns held tokens against bad debt, routes cash to NSM and offers residual tokens through an oracle-free auction. Auction pricing and competition remain economic risks.

Run `python3 -m analysis.lifecycle_study` to regenerate `lifecycle-results.json`, or add `--check` to verify it. The unittest command above also runs the lifecycle suite. This module remains separate from the signed Rust vault ledger; the [integration contract](../docs/reserve-rust-contract.md) specifies that next step.
