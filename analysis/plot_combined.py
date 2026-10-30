"""Plot deterministic signed-ledger outcomes; no market calibration or forecasts."""
import json
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parent
MICRO = 1_000_000


def cases(rows, liquidity, timing):
    selected = [row for row in rows
                if row['oracle_lag_blocks'] == 0
                and row['initial_reserve_inventory_micro_zusd'] == 200 * MICRO
                and row['liquidator_available_micro_zusd'] == liquidity * MICRO
                and row['liquidation_timing'] == timing]
    selected.sort(key=lambda row: row['true_zec_usd'])
    if len(selected) != 6:
        raise ValueError('expected the six distinct crash-price cases')
    return selected


def main():
    data = json.loads((ROOT / 'combined-results.json').read_text())
    rows = data['liquidation']
    early = cases(rows, 4000, 'on_report')
    late = cases(rows, 4000, 'closing_block')
    absent = cases(rows, 0, 'on_report')
    prices = [row['true_zec_usd'] for row in early]
    if any(row['recorded_bad_debt_micro_zusd'] for row in absent):
        raise ValueError('no-liquidator annotation no longer matches the results')

    plt.rcParams.update({'font.size': 11, 'axes.spines.top': False,
                         'axes.spines.right': False, 'svg.fonttype': 'none'})
    fig, axes = plt.subplots(1, 2, figsize=(12, 5.3), constrained_layout=False)
    fig.subplots_adjust(top=0.78, bottom=0.24, left=0.08, right=0.97, wspace=0.27)
    fig.suptitle('Identical collateral can leave different recorded losses',
                 x=0.08, y=0.96, ha='left', fontsize=17, weight='bold')
    fig.text(0.08, 0.89, 'Signed-ledger stress cases · simulated funds · fixed post-crash prices',
             color='#555555')

    for series, label, color, marker in [
        (early, 'Liquidate before closure (block 2)', '#2166AC', 'o'),
        (late, 'Liquidate in closing block (block 8)', '#B35806', 's'),
    ]:
        axes[0].plot(prices, [r['recorded_bad_debt_micro_zusd'] / MICRO for r in series],
                     label=label, color=color, marker=marker, linewidth=2)
    axes[0].set_title('Available liquidator tokens: 4,000 zUSD', loc='left', fontsize=12)
    axes[0].set_ylabel('Recorded bad debt after closure (zUSD)')
    axes[0].legend(frameon=False, fontsize=9, loc='upper right')

    axes[1].plot(prices, [r['active_target_principal_shortfall_micro_usd'] / MICRO
                          for r in absent], color='#762A83', marker='o', linewidth=2)
    axes[1].set_title('No liquidator submits a transaction', loc='left', fontsize=12)
    axes[1].set_ylabel('Active principal shortfall at $1 target (USD)')
    axes[1].text(0.98, 0.92, 'Recorded bad debt stays zero.\nThe vault remains open.',
                 transform=axes[1].transAxes, ha='right', va='top', fontsize=10)
    for axis in axes:
        axis.set_xlabel('Assumed ZEC price after crash (USD)')
        axis.set_xticks(prices)
        axis.set_ylim(bottom=0, top=810)
        axis.grid(axis='y', alpha=0.2)
    fig.text(0.08, 0.10, 'Target vault: 25 ZEC collateral, 1,000 zUSD principal. Reserve inventory: 200 zUSD.\n'
             'Closure burns inventory once, before user transactions. No fees or endogenous price response.\n'
             'Dots are executed cases; connecting lines are visual guides. Source: analysis/combined-results.json.',
             fontsize=9, color='#555555', va='center')
    fig.savefig(ROOT / 'combined-stress.png', dpi=180)
    plt.close(fig)


if __name__ == '__main__':
    main()
