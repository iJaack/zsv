"""Independently recompute the signed study's fixed fixture arithmetic.

This checks reported results against the documented scenario assumptions. It
neither executes the Rust ledger nor establishes market prices or participation.
"""
from fractions import Fraction
from itertools import product
import json
from pathlib import Path

ZAT = 10**8
MICRO = 10**6


def equal(actual, expected, context):
    if actual != expected:
        raise ValueError(f'{context}: expected {expected!r}, received {actual!r}')


def fields(case, expected, context):
    for name, value in expected.items():
        equal(case[name], value, f'{context}.{name}')


def grid(rows, keys, axes, name):
    actual = {tuple(row[key] for key in keys) for row in rows}
    expected = set(product(*axes))
    equal(actual, expected, f'{name} parameter combinations')
    equal(len(rows), len(expected), f'{name} unique rows')


def usd_micro(zatoshi, price):
    return int(Fraction(zatoshi * price * MICRO, ZAT))


def liquidation(rows):
    grid(rows, ['true_zec_usd', 'oracle_lag_blocks',
                'liquidator_available_micro_zusd', 'initial_reserve_inventory_micro_zusd',
                'liquidation_timing'],
         [[10, 20, 40, 60, 80, 100], [0, 2, 4], [0, 100*MICRO, 4000*MICRO],
          [0, 200*MICRO, 800*MICRO], ['on_report', 'closing_block']], 'liquidation')
    for case in rows:
        price = case['true_zec_usd']
        available = case['liquidator_available_micro_zusd']
        inventory = case['initial_reserve_inventory_micro_zusd']
        if available == 0:
            outcome = 'no_liquidator'
        elif price >= 80:
            outcome = 'not_liquidatable'
        elif available == 100*MICRO:
            outcome = 'insufficient_liquidator_tokens'
        else:
            outcome = 'liquidated'
        closed = outcome == 'liquidated'
        capacity = int(Fraction(25 * price * MICRO, Fraction(105, 100)))
        burn = min(1000*MICRO, capacity) if closed else 0
        bad_debt = 1000*MICRO - burn if closed else 0
        cover = min(inventory, bad_debt) if case['liquidation_timing'] == 'on_report' else 0
        principal = 0 if closed else 1000*MICRO
        report_height = 2 + case['oracle_lag_blocks']
        attempt_height = 8 if case['liquidation_timing'] == 'closing_block' else report_height
        # Stocking occurs at $100/ZEC and a $0.995 bid, before the assumed crash.
        stock_cost = int(Fraction(inventory, MICRO) * Fraction(995, 1000) * ZAT / 100)
        fields(case, {
            'outcome': outcome,
            'oracle_report_height': report_height,
            'attempt_height': attempt_height,
            'recorded_bad_debt_micro_zusd': bad_debt-cover,
            'shutdown_burn_micro_zusd': cover,
            'remaining_reserve_inventory_micro_zusd': inventory-cover,
            'active_target_principal_micro_zusd': principal,
            'active_target_principal_shortfall_micro_usd': max(0, principal-25*price*MICRO),
            'target_principal_shortfall_at_crash_micro_usd': max(0, 1000*MICRO-25*price*MICRO),
            'target_bonus_adjusted_shortfall_at_crash_micro_zusd': max(0, 1000*MICRO-capacity),
            'elapsed_blocks_with_unsettled_target_since_true_price_change': attempt_height-2 if closed else 6,
            'stable_supply_micro_zusd': 5000*MICRO-burn-cover,
            'nsm_zatoshi': 100*ZAT-stock_cost,
        }, f'liquidation(price={price}, available={available}, inventory={inventory}, height={attempt_height})')


def stale_quotes(rows):
    grid(rows, ['true_zec_usd', 'side'], [[20, 100, 200],
         ['reserve_sells_tokens', 'reserve_buys_tokens']], 'stale quotes')
    for case in rows:
        sells = case['side'] == 'reserve_sells_tokens'
        flow = 402_000_000 if sells else 398_000_000
        value = usd_micro(flow, case['true_zec_usd'])
        fields(case, {
            'reported_zec_usd': 100,
            'resigned_old_price_at_height': 6,
            'traded_micro_zusd': 400*MICRO,
            'zec_flow_zatoshi': flow,
            'zec_flow_at_true_price_micro_usd': value,
            'trader_quote_advantage_if_token_worth_one_usd_micro_usd': 400*MICRO-value if sells else value-400*MICRO,
            'active_target_principal_shortfall_micro_usd': max(0, 1000*MICRO-25*case['true_zec_usd']*MICRO),
        }, f'stale quote({case["side"]}, price={case["true_zec_usd"]})')


def funding(rows):
    grid(rows, ['retained_funding_cap_zatoshi', 'funding_per_block_zatoshi', 'true_zec_usd'],
         [[10*ZAT, 12*ZAT, 20*ZAT], [0, ZAT], [20, 100]], 'funding')
    for case in rows:
        cap = case['retained_funding_cap_zatoshi']
        allocation = case['funding_per_block_zatoshi']
        price = case['true_zec_usd']
        cash = allocated = 10*ZAT
        nsm = spent = fills = 0
        cost = int(Fraction(500*995*ZAT, 1000*100))
        for height in range(1, 11):
            if height == 10:
                nsm += cash
                cash = 0
            retained = min(allocation, 10*ZAT-cash, cap-allocated) if height < 10 else 0
            cash += retained
            allocated += retained
            nsm += allocation-retained
            if height < 10 and cash >= cost:
                cash -= cost
                spent += cost
                fills += 1
        equal(spent+nsm, 10*ZAT+allocation*10, 'funding ZEC conservation')
        fields(case, {
            'accepted_500_token_purchases': fills,
            'cash_rejected_purchases': 9-fills,
            'initial_reserve_zatoshi': 10*ZAT,
            'retained_initial_and_new_funding_zatoshi': allocated,
            'scheduled_funding_zatoshi': allocation*10,
            'nsm_zatoshi': nsm,
            'gross_purchase_payout_zatoshi': spent,
            'unauctioned_inventory_micro_zusd': fills*500*MICRO,
            'gross_purchase_payout_at_true_price_micro_usd': usd_micro(spent, price),
            'nsm_cash_at_true_price_micro_usd': usd_micro(nsm, price),
            'all_public_appropriations_at_true_price_micro_usd': usd_micro(10*ZAT+allocation*10, price),
            'active_target_principal_shortfall_micro_usd': max(0, 10000*MICRO-250*price*MICRO),
        }, f'funding(cap={cap}, allocation={allocation}, price={price})')


def auctions(rows):
    grid(rows, ['true_zec_usd', 'participation'], [[20, 100, 200],
         ['no_bidders', 'only_borrower_bids_at_floor',
          'par_valuing_buyer_at_first_affordable_block']], 'auctions')
    for case in rows:
        price = case['true_zec_usd']
        participation = case['participation']
        height = None
        ask = 0
        inventory = 400*MICRO
        pnl = None
        principal = 1000*MICRO
        if participation == 'only_borrower_bids_at_floor':
            height, ask, inventory = 2003, 400, 0
            pnl, principal = 398_000_000-ask, 0
        elif participation == 'par_valuing_buyer_at_first_affordable_block':
            for elapsed in range(1, 2001):
                quote = Fraction((ZAT-1)*(2000-elapsed), 2000)
                unit_price = 1 + (-(-quote.numerator // quote.denominator))
                ask = 400*unit_price
                if Fraction(ask*price, ZAT) <= 400:
                    height, inventory = elapsed+3, 0
                    break
        proceeds = usd_micro(ask, price)
        fields(case, {
            'auction_duration_blocks': 2000,
            'sale_height': height,
            'auction_proceeds_zatoshi': ask,
            'auction_proceeds_at_true_price_micro_usd': proceeds,
            'unauctioned_inventory_micro_zusd': inventory,
            'nsm_zatoshi': 100*ZAT-398_000_000+ask,
            'borrower_complete_cycle_pnl_zatoshi': pnl,
            'borrower_complete_cycle_pnl_at_true_price_micro_usd': None if pnl is None else usd_micro(pnl, price),
            'active_target_principal_micro_zusd': principal,
            'conditional_buyer_discount_to_one_usd_micro_usd': None if height is None else 400*MICRO-proceeds,
        }, f'auction({participation}, price={price})')


def main():
    study = json.loads(Path(__file__).with_name('combined-results.json').read_text())
    equal(study['case_counts'], {'liquidation': 324, 'stale_quotes': 6,
          'funding': 12, 'auctions': 9}, 'case counts')
    liquidation(study['liquidation'])
    stale_quotes(study['stale_quotes'])
    funding(study['funding'])
    auctions(study['auctions'])
    print('Independent integer/Fraction checks passed for all 351 combined cases.')


if __name__ == '__main__':
    main()
