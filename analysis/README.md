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
