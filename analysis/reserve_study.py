"""Reproduce the inventory reserve study; prices and participation are assumptions."""

import argparse
import csv
from dataclasses import asdict, dataclass, replace
from fractions import Fraction
import io
import itertools
import json
from pathlib import Path

from analysis.reserve import (MICRO, U64_MAX, ZAT, Order, Policy, Prices, Wallet,
                              bootstrap, credit_funding, execute)

ROOT = Path(__file__).resolve().parents[1]


def fill_largest(state, policy, prices, side, desired, height):
    """Find the largest atomic fill; rejected trials never mutate the ledger."""
    def attempt(quantity):
        order = Order('market', side, quantity, state.accounts['market'].nonce,
                      prices.zec_sequence, height // policy.epoch_blocks, height,
                      0 if side == 'buy' else U64_MAX)
        return execute(state, policy, prices, order, height)

    minimum = 1
    if side == 'buy':
        denominator = (MICRO - policy.spread) * ZAT
        minimum = (MICRO * prices.zec_usd + denominator - 1) // denominator
    if desired < minimum:
        return state, None
    try:
        best = attempt(minimum)
    except ValueError:
        return state, None
    low, high = minimum + 1, desired
    while low <= high:
        middle = (low + high) // 2
        try:
            best = attempt(middle)
            low = middle + 1
        except ValueError:
            high = middle - 1
    return best


@dataclass(frozen=True)
class MarketCase:
    name: str
    demand: int = 75_000
    reserve_zec: int = 100
    inventory: int = 10_000
    zec_usd: int = 100
    epochs: int = 40
    response: Fraction = Fraction(1)
    demand_step: int = 0
    funding_zat: int = 0
    hurdle: Fraction = Fraction(0)
    sunset: int = 1000
    lifetime_zec_cap: int = 100 * ZAT
    epoch_token_cap: int = 1_000 * MICRO


def simulate_market(case):
    """All nonreserve tokens are assumed circulating; no vault response occurs."""
    policy = Policy(sunset_height=case.sunset, lifetime_zec_cap=case.lifetime_zec_cap,
                    epoch_token_cap=case.epoch_token_cap)
    state = bootstrap({'donor': Wallet(case.reserve_zec * ZAT, case.inventory * MICRO),
                       'market': Wallet(10_000 * ZAT, 80_000 * MICRO)},
                      'donor', case.reserve_zec * ZAT, case.inventory * MICRO, policy)
    demand, depth = case.demand, 100_000
    bid = Fraction(MICRO - policy.spread, MICRO)
    ask = Fraction(MICRO + policy.spread, MICRO)
    rows = []
    for epoch in range(case.epochs + 1):
        height = epoch * policy.epoch_blocks
        float_units = state.stable_supply - state.reserve_tokens
        price = 1 + (Fraction(demand) - Fraction(float_units, MICRO)) / depth
        if price < 0:
            raise ValueError('linear market left its nonnegative price domain')
        side, desired, receipt = '', 0, None
        before = state
        if epoch < case.epochs and height < policy.sunset_height:
            state = credit_funding(state, policy, case.funding_zat, height)
            prices = Prices(case.zec_usd * MICRO, epoch + 1, height)
            if price > ask + case.hurdle:
                side, gap = 'sell', price - ask - case.hurdle
            elif price < bid - case.hurdle:
                side, gap = 'buy', bid - case.hurdle - price
            if side:
                desired = int(gap * depth * MICRO * case.response)
                state, receipt = fill_largest(state, policy, prices, side, desired, height)
        rows.append({'scenario': case.name, 'epoch': epoch, 'price': float(price),
                     'float_tokens': float_units / MICRO,
                     'reserve_zec': before.reserve_zec / ZAT,
                     'inventory_tokens': before.reserve_tokens / MICRO,
                     'total_supply_tokens': before.stable_supply / MICRO,
                     'nsm_zec': before.nsm_zec / ZAT, 'side': side,
                     'desired_tokens': desired / MICRO,
                     'filled_tokens': receipt['tokens'] / MICRO if receipt else 0,
                     'zec_exchanged': receipt['zec_amount'] / ZAT if receipt else 0})
        assert state.stable_supply == before.stable_supply
        assert (state.active_debt, state.bad_debt) == (before.active_debt, before.bad_debt)
        demand += case.demand_step
    return rows


def cases():
    base = MarketCase('funded_purchase')
    return [base, replace(base, name='funded_sale', demand=85_000),
            replace(base, name='underfunded_purchase', reserve_zec=10),
            replace(base, name='zec_crash_purchase', zec_usd=20),
            replace(base, name='empty_sale_inventory', demand=85_000, inventory=0),
            replace(base, name='small_sale_inventory', demand=85_000, inventory=1_000),
            replace(base, name='demand_exceeds_total_supply', demand=100_000),
            replace(base, name='no_participation', response=Fraction(0)),
            replace(base, name='transaction_hurdle', hurdle=Fraction(1, 100)),
            replace(base, name='early_sunset', sunset=20),
            replace(base, name='persistent_contraction', demand=80_000, demand_step=-500,
                    funding_zat=ZAT // 5, lifetime_zec_cap=1000 * ZAT)]


def summarize(rows):
    return {'initial_price': rows[0]['price'], 'final_price': rows[-1]['price'],
            'final_inventory_tokens': rows[-1]['inventory_tokens'],
            'final_reserve_zec': rows[-1]['reserve_zec'], 'nsm_zec': rows[-1]['nsm_zec'],
            'purchased_tokens': sum(r['filled_tokens'] for r in rows if r['side'] == 'buy'),
            'sold_tokens': sum(r['filled_tokens'] for r in rows if r['side'] == 'sell'),
            'zec_paid': sum(r['zec_exchanged'] for r in rows if r['side'] == 'buy'),
            'zec_received': sum(r['zec_exchanged'] for r in rows if r['side'] == 'sell')}


def hostile_fill(side, true_zec, true_token=Fraction(1)):
    policy = Policy(epoch_zec_cap=100 * ZAT, epoch_token_cap=10_000 * MICRO)
    state = bootstrap({'donor': Wallet(100 * ZAT, 10_000 * MICRO),
                       'market': Wallet(1000 * ZAT, 20_000 * MICRO)},
                      'donor', 100 * ZAT, 10_000 * MICRO, policy)
    original = state
    state, receipt = execute(state, policy, Prices(100 * MICRO, 1, 0),
                             Order('market', side, 10_000 * MICRO, 0, 1, 0, 0,
                                   0 if side == 'buy' else U64_MAX), 0)
    old = original.accounts['market']
    new = state.accounts['market']
    gain = Fraction((new.zec - old.zec) * true_zec, ZAT)
    gain += Fraction(new.tokens - old.tokens, MICRO) * true_token
    return {'side': side, 'tokens': receipt['tokens'] / MICRO,
            'zec_exchanged': receipt['zec_amount'] / ZAT,
            'reported_zec_usd': 100, 'true_zec_usd': true_zec,
            'true_token_usd': float(true_token), 'trader_gain_usd': float(gain),
            'reserve_plus_nsm_trade_loss_usd': float(gain)}


def generate():
    traces, results = [], {}
    for case in cases():
        rows = simulate_market(case)
        assumptions = asdict(case)
        assumptions.update(response=str(case.response), hurdle=str(case.hurdle))
        results[case.name] = {'assumptions': assumptions, 'result': summarize(rows)}
        traces.extend(rows)
    sensitivity = []
    for reserve, inventory, zec_price, demand, sale_cap in itertools.product(
            (0, 10, 50, 100), (0, 1000, 10_000), (20, 100), (75_000, 85_000),
            (100, 1000, 5000)):
        case = MarketCase('grid', reserve_zec=reserve, inventory=inventory, zec_usd=zec_price,
                          demand=demand, epoch_token_cap=sale_cap * MICRO)
        sensitivity.append({'reserve_zec': reserve, 'inventory': inventory,
                            'zec_usd': zec_price, 'demand': demand,
                            'epoch_sale_cap_tokens': sale_cap, **summarize(simulate_market(case))})
    outputs = {'scope': 'Synthetic fixtures and uncalibrated deterministic market assumptions.',
               'scenarios': results, 'sensitivity_runs': len(sensitivity),
               'hostile_trades': {
                   'zec_gap_down': hostile_fill('sell', 20),
                   'zec_gap_up': hostile_fill('buy', 200),
                   'fundamental_token_depeg': hostile_fill('buy', 100, Fraction(1, 2))},
               'initial_total_supply_tokens_base': 90_000,
               'initial_inventory_target_value_usd_base': 10_000,
               'initial_cash_usd_base': 10_000}
    return outputs, traces, sensitivity


def csv_text(rows):
    stream = io.StringIO(newline='')
    writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator='\n')
    writer.writeheader()
    writer.writerows(rows)
    return stream.getvalue()


def plot(traces):
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    plt.rcParams.update({'font.size': 10, 'axes.spines.top': False, 'axes.spines.right': False})
    fig, axes = plt.subplots(1, 2, figsize=(11, 4.5))
    panels = [
        (axes[0], 'Purchases below target', ('funded_purchase', 'underfunded_purchase', 'zec_crash_purchase'),
         ('100 ZEC at $100', '10 ZEC at $100', '100 ZEC at $20')),
        (axes[1], 'Sales above target', ('funded_sale', 'small_sale_inventory', 'empty_sale_inventory'),
         ('10,000 tokens', '1,000 tokens', 'No token inventory'))]
    for ax, title, names, labels in panels:
        for name, label, color, style in zip(names, labels, ('#0072B2', '#D55E00', '#009E73'), ('-', '--', ':')):
            rows = [r for r in traces if r['scenario'] == name]
            ax.plot([r['epoch'] for r in rows], [r['price'] for r in rows],
                    label=label, color=color, linestyle=style, linewidth=1.8)
        ax.axhspan(.995, 1.005, color='#999999', alpha=.12, label='Quoted spread')
        ax.set(title=title, xlabel='Epoch', ylabel='USD / stablecoin')
        ax.grid(alpha=.2)
        ax.legend(fontsize=8)
    fig.suptitle('Inventory reserve: capital and inventory constrain support')
    fig.text(.5, .01, 'Assumed market arbitrage and linear depth; no price forecast, vault response or guaranteed redemption.',
             ha='center', fontsize=8.5)
    fig.tight_layout(rect=(0, .05, 1, .94))
    fig.savefig(ROOT / 'docs' / 'figures' / 'reserve-scenarios.png', dpi=160)
    plt.close(fig)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--plots', action='store_true')
    args = parser.parse_args()
    if args.check and args.plots:
        parser.error('--check and --plots are separate operations')
    outputs, traces, sensitivity = generate()
    files = {'reserve-results.json': json.dumps(outputs, indent=2, sort_keys=True, allow_nan=False) + '\n',
             'reserve-trajectories.csv': csv_text(traces), 'reserve-sensitivity.csv': csv_text(sensitivity)}
    for name, content in files.items():
        path = ROOT / 'analysis' / name
        if args.check:
            if not path.exists() or path.read_text() != content:
                raise SystemExit(f'Stale reserve output: {name}')
        else:
            path.write_text(content)
    if args.plots:
        plot(traces)
    print(f"Reserve study {'checked' if args.check else 'generated'}: "
          f"{len(outputs['scenarios'])} scenarios, {len(sensitivity)} parameter combinations.")


if __name__ == '__main__':
    main()
