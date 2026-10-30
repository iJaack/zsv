"""Regenerate conditional model results; use --check to detect stale outputs."""

import argparse
import csv
from dataclasses import asdict, replace
from fractions import Fraction
import io
import itertools
import json
from pathlib import Path

from attacks import farm_profit, pool_displacement, proportional_payout, reserve_capacity
from stabilization import Scenario, ideal_errors, signed_budget_difference, simulate, split_cut, tapered_share

ROOT = Path(__file__).resolve().parents[1]
ZATOSHI = 100_000_000


def summary(rows):
    return {
        'final_price_usd': rows[-1]['price'],
        'final_supply': rows[-1]['supply'],
        'final_reserve_zec': rows[-1]['reserve_zec'],
        'payout_usd': sum(row['payout_usd'] for row in rows),
        'absolute_action_tokens': sum(abs(row['action']) for row in rows),
    }


def scenarios():
    base = Scenario(name='funded_expansion', epochs=40)
    return [base,
            replace(base, name='underfunded_expansion', reserve_zec=.1),
            replace(base, name='no_response', response=0),
            replace(base, name='no_collateral_headroom', collateral_zec=2000),
            replace(base, name='bad_debt_freeze', bad_debt=True),
            replace(base, name='funded_contraction', target_demand=75_000),
            replace(base, name='underfunded_contraction', target_demand=75_000, reserve_zec=.1),
            replace(base, name='zec_crash_contraction', target_demand=75_000, zec_usd=20),
            replace(base, name='persistent_demand', reserve_zec=.1, funding_zec=.02,
                    demand_step_per_epoch=500),
            replace(base, name='delayed_overshoot', delay=1, gain=1.1)]


def csv_text(rows):
    output = io.StringIO(newline='')
    writer = csv.DictWriter(output, fieldnames=list(rows[0]), lineterminator='\n')
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue()


def study():
    funding = []
    for month in (0, 1, 12, 24, 48, 72):
        parts = split_cut(1_000_000 * ZATOSHI, Fraction(2, 25),
                          tapered_share(month), Fraction(1, 10))
        funding.append({'month': month,
                        'grants_pct_subsidy': float(100 * tapered_share(month)),
                        'incentives_pct_subsidy': float(Fraction(parts['incentives'], 10**12)),
                        'per_million_subsidy_zec': {key: value / ZATOSHI for key, value in parts.items()}})

    # A normalization example, not a reconstruction of the forum's schedule.
    subsidies = [(54_000 if month < 24 else 27_000) * ZATOSHI for month in range(72)]
    current = [Fraction(2, 25) if month < 24 else Fraction(0) for month in range(72)]
    renewed = [Fraction(2, 25)] * 72
    proposed = [tapered_share(month) for month in range(72)]
    differences = {label: signed_budget_difference(subsidies, shares, proposed) / ZATOSHI
                   for label, shares in [('current_law', current), ('renewed_eight_percent', renewed)]}

    outputs = {}
    traces = []
    configs = scenarios()
    for config in configs:
        rows = simulate(config)
        outputs[config.name] = {'assumptions': asdict(config), 'result': summary(rows)}
        traces.extend({'scenario': config.name, **row} for row in rows)

    sensitivity = []
    for gain, delay, response, reserve, price in itertools.product(
            (.25, .5, .75, 1, 1.5), (0, 1), (0, .25, 1), (.1, .5, 1, 2), (20, 100)):
        config = replace(configs[0], gain=gain, delay=delay, response=response,
                         reserve_zec=reserve, zec_usd=price)
        rows = simulate(config)
        for row in rows:
            assert row['reserve_zec'] >= -1e-10
            assert abs(row['supply'] - row['debt']) < 1e-8
            assert 0 <= row['supply'] <= config.debt_ceiling + 1e-8
        item = {'gain': gain, 'delay': delay, 'response': response,
                'reserve_zec': reserve, 'zec_usd': price, **summary(rows)}
        item['within_one_cent_after_40_epochs'] = abs(item['final_price_usd'] - 1) < .01
        sensitivity.append(item)

    attacks = {
        'retained_mint_burn_one_day_profit_usd': farm_profit(10_000, .01, .01, 2.5, .1, 1, 1),
        'retained_mint_burn_thirty_day_profit_usd': farm_profit(10_000, .01, .01, 2.5, .1, 30, 1),
        'break_even_holding_days_without_fixed_cost': .02 / (2.5 * .1) * 365,
        'wallet_cap_one_wallet_payout_usd': float(proportional_payout(Fraction(100), 1000, 100)),
        'wallet_cap_ten_wallets_payout_usd': float(proportional_payout(Fraction(100), 1000, 1000)),
        'thin_pool_two_percent_upward_price_move': pool_displacement(1000, 1000, 1.02),
        'bounty_capacity_at_20_usd_zec': reserve_capacity(1000, 20, .02),
        'bounty_capacity_at_10_usd_zec': reserve_capacity(1000, 10, .02),
        'purchase_capacity_at_20_usd_zec_one_dollar_token': reserve_capacity(1000, 20, 1),
    }
    results = {
        'scope': 'Uncalibrated deterministic scenarios. No rewards implemented in Rust. No forecasts.',
        'funding': {'hypothetical_renewed_eight_percent_reference': funding,
                    'normalization_example_net_cut_zec': differences,
                    'forum_rounded_future_cut_zec': 109_000 - 47_000,
                    'ten_percent_of_forum_future_cut_zec': (109_000 - 47_000) / 10,
                    'treasury_donation_included': False},
        'scenarios': outputs,
        'adversarial_examples': attacks,
        'sensitivity_runs': len(sensitivity),
        'sensitivity_within_one_cent_after_40_epochs': sum(
            item['within_one_cent_after_40_epochs'] for item in sensitivity),
    }
    return results, traces, sensitivity


def plots(traces):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    plt.rcParams.update({'font.size': 10, 'axes.spines.top': False,
                         'axes.spines.right': False, 'savefig.dpi': 160})
    folder = ROOT / 'docs' / 'figures'
    folder.mkdir(exist_ok=True)
    colors = ('#0072B2', '#D55E00', '#009E73')
    styles = ('-', '--', ':')
    fig, axes = plt.subplots(1, 2, figsize=(11, 4), sharey=True)
    for ax, delay, gains in zip(axes, (0, 1), ((.5, 1.5, 2), (.5, 1, 1.1))):
        for gain, color, style in zip(gains, colors, styles):
            errors = ideal_errors(gain, delay, .05, 24)
            ax.plot(range(25), [100 * error for error in errors],
                    label=f'Gain {gain:g}', color=color, linestyle=style, linewidth=1.8)
        ax.axhline(0, color='#555555', linewidth=.6)
        ax.set(title=f'{delay} epoch response delay', xlabel='Epoch')
        ax.grid(alpha=.2)
        ax.legend()
    axes[0].set_ylabel('Price error (% of $1 target)')
    fig.suptitle('Linear recurrence: delay changes the stability limit')
    fig.text(.5, .01, 'Model equations only; unlimited response and funding. No probability estimates.',
             ha='center', fontsize=9)
    fig.tight_layout(rect=(0, .05, 1, .94))
    fig.savefig(folder / 'control-stability.png')
    plt.close(fig)

    grouped = {name: [row for row in traces if row['scenario'] == name]
               for name in {row['scenario'] for row in traces}}
    fig, axes = plt.subplots(2, 2, figsize=(11, 7))
    panels = [
        (axes[0, 0], 'Above target: one-off demand shock', 'price',
         ('funded_expansion', 'underfunded_expansion', 'no_response'),
         ('1 ZEC reserve', '0.1 ZEC reserve', 'No willing response')),
        (axes[0, 1], 'Below target: one-off demand shock', 'price',
         ('funded_contraction', 'underfunded_contraction', 'zec_crash_contraction'),
         ('1 ZEC at $100', '0.1 ZEC at $100', '1 ZEC at $20')),
        (axes[1, 0], 'Persistent demand: 500 more tokens / epoch', 'price',
         ('persistent_demand',), ('0.02 ZEC inflow / epoch',)),
        (axes[1, 1], 'Spendable reserve after completed epochs', 'reserve_zec',
         ('funded_expansion', 'underfunded_expansion', 'persistent_demand'),
         ('Funded one-off shock', 'Underfunded one-off shock', 'Persistent shock')),
    ]
    for ax, title, metric, names, labels in panels:
        for name, label, color, style in zip(names, labels, colors, styles):
            rows = grouped[name]
            ax.plot([row['epoch'] for row in rows], [row[metric] for row in rows],
                    label=label, color=color, linestyle=style, linewidth=1.8)
        if metric == 'price':
            ax.axhline(1, color='#555555', linewidth=.6)
        ax.set(title=title, xlabel='Epoch', ylabel='USD / stablecoin' if metric == 'price' else 'ZEC')
        ax.grid(alpha=.2)
        ax.legend(fontsize=8)
    fig.suptitle('Finite-budget scenarios: $0.01 borrower bounty, gain 0.5')
    fig.text(.5, .01, 'Assumed honest response; no liquidation, market slippage or price forecast. Full assumptions in results.json.',
             ha='center', fontsize=9)
    fig.tight_layout(rect=(0, .04, 1, .95))
    fig.savefig(folder / 'stabilization-scenarios.png')
    plt.close(fig)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--plots', action='store_true')
    args = parser.parse_args()
    if args.check and args.plots:
        parser.error('--check and --plots are separate operations')
    results, traces, sensitivity = study()
    files = {'results.json': json.dumps(results, indent=2, sort_keys=True, allow_nan=False) + '\n',
             'trajectories.csv': csv_text(traces), 'sensitivity.csv': csv_text(sensitivity)}
    for name, value in files.items():
        path = ROOT / 'analysis' / name
        if args.check:
            if not path.exists() or path.read_text() != value:
                raise SystemExit(f'Stale study output: {path.relative_to(ROOT)}')
        else:
            path.write_text(value)
    if args.plots:
        plots(traces)
    print(f"Study {'checked' if args.check else 'generated'}: "
          f"{len(results['scenarios'])} scenarios, {len(sensitivity)} sensitivity runs.")


if __name__ == '__main__':
    main()
