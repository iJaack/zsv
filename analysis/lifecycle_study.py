"""Reproduce exact lifecycle cases; synthetic balances, no market forecast."""

import argparse
from dataclasses import asdict, replace
import json
from pathlib import Path

from analysis.reserve import MICRO, U64_MAX, ZAT, Order, Prices, Wallet
from analysis.reserve_lifecycle import (Config, bootstrap, on_block, execute,
                                       credit_funding, wind_down_buy)


def fixture(config=Config(), bad_debt=0):
    return bootstrap({'donor': Wallet(100 * ZAT, 10_000 * MICRO),
                      'market': Wallet(1000 * ZAT, 80_000 * MICRO)},
                     'donor', 100 * ZAT, 10_000 * MICRO, config, bad_debt=bad_debt)


def advance(state, height, fresh=True):
    for block in range(state.ledger.height + 1, height + 1):
        state = on_block(state, block, Prices(100 * MICRO, block + 1, block) if fresh else None)
    return state


def swap(state, side, quantity):
    height = state.ledger.height
    order = Order('market', side, quantity * MICRO, state.ledger.accounts['market'].nonce,
                  state.latest_prices.zec_sequence, state.ledger.epoch, height,
                  0 if side == 'buy' else U64_MAX)
    return execute(state, order, height)[0]


def snapshot(state):
    ledger = state.ledger
    return {'reserve_zat': ledger.reserve_zec, 'reserve_micro_tokens': ledger.reserve_tokens,
            'retained_funding_zat': state.retained_funding_zec,
            'gross_zat_paid': ledger.lifetime_zec_out, 'nsm_zat': ledger.nsm_zec,
            'stable_supply_micro': ledger.stable_supply, 'active_debt_micro': ledger.active_debt,
            'bad_debt_micro': ledger.bad_debt, 'normal_quotes_closed': state.closed,
            'shutdown': asdict(state.shutdown) if state.shutdown else None}


def study():
    # Same 20-epoch cycle that exhausted v1 lifetime capacity.
    state = on_block(fixture(), 0, Prices(100 * MICRO, 1, 0))
    for epoch in range(20):
        state = advance(state, epoch * 10)
        state = swap(state, 'sell' if epoch < 10 else 'buy', 1000)
    recycled = snapshot(state)
    state = advance(state, 200)
    state = swap(state, 'buy', 100)
    after_genuine_order = snapshot(state)

    # Initial100 + retained19.9+19.9+10.2 reaches150; other credits go to NSM.
    config = replace(Config(), lifetime_retained_funding_cap=150 * ZAT)
    state = on_block(fixture(config), 0, Prices(100 * MICRO, 1, 0))
    allocation_rows = []
    for index, credit in enumerate((30, 50, 50)):
        for offset in range(2):
            state = advance(state, (index * 2 + offset) * 10)
            state = swap(state, 'buy', 1000)
        state = credit_funding(state, credit * ZAT, state.ledger.height)
        allocation_rows.append(snapshot(state))

    # Close with6000 bad debt; retain4000 tokens for an oracle-free paid exit.
    config = replace(Config(), policy=replace(Config().policy, sunset_height=1))
    state = on_block(fixture(config, bad_debt=6000 * MICRO), 0, Prices(100 * MICRO, 1, 0))
    state = on_block(state, 1)
    retired = snapshot(state)
    floor_height = 1 + config.auction_duration_blocks
    state = on_block(state, floor_height)
    state, fill = wind_down_buy(state, 'market', 4000 * MICRO,
                               state.ledger.accounts['market'].nonce, U64_MAX, floor_height)
    auction = {'after': snapshot(state), 'fill': fill,
               'note': 'Worst-case floor fill, not an estimate of competitive clearing price.'}

    # Always-stale newer reports cannot keep the reserve alive.
    state = on_block(fixture(), 0, Prices(100 * MICRO, 1, 0))
    state = on_block(state, 4)
    for height in range(5, 11):
        state = on_block(state, height, Prices(100 * MICRO, height + 1, height - 4))
    outage = snapshot(state)

    return {'scope': 'Exact accounting cases; host-authorized synthetic inputs.',
            'after_round_trip_cycle': recycled,
            'subsequent_purchase': after_genuine_order,
            'retained_allocation_path': allocation_rows,
            'shutdown_before_auction': retired, 'auction_at_floor': auction,
            'stale_update_trickle': outage}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = json.dumps(study(), sort_keys=True, indent=2) + '\n'
    path = Path(__file__).with_name('lifecycle-results.json')
    if args.check:
        if not path.exists() or path.read_text() != result:
            raise SystemExit('Stale lifecycle numeric output')
    else:
        path.write_text(result)
    print('Lifecycle accounting cases reproduced.' if args.check else 'Lifecycle accounting cases generated.')


if __name__ == '__main__':
    main()
