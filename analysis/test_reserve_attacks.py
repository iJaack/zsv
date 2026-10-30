"""Independent economic counterexamples for the standing-quote reserve.

True prices below are assumptions used to value completed transfers. The core
sees only the canonical simulation observation; these tests add no price feed.
"""

from dataclasses import replace
from fractions import Fraction
import unittest

from analysis.reserve import (MICRO, U64_MAX, ZAT, Order, Policy, Prices,
                              Wallet, bootstrap, credit_funding, execute)


class ReserveAttackTests(unittest.TestCase):
    def initial(self, policy):
        return bootstrap(
            {'donor': Wallet(1000 * ZAT, 20_000 * MICRO),
             'trader': Wallet(1000 * ZAT, 20_000 * MICRO)},
            'donor', 100 * ZAT, 10_000 * MICRO, policy)

    def fill(self, state, policy, prices, side, tokens, height=0):
        order = Order('trader', side, tokens, state.accounts['trader'].nonce,
                      prices.zec_sequence, height // policy.epoch_blocks,
                      height, 0 if side == 'buy' else U64_MAX)
        return execute(state, policy, prices, order, height)

    def public_value(self, state, true_zec_price, true_token_price=1):
        # Funding overflow remains public assets outside reserve working cash.
        return (Fraction(state.reserve_zec + state.nsm_zec, ZAT) * true_zec_price
                + Fraction(state.reserve_tokens, MICRO) * true_token_price)

    def trader_value(self, state, true_zec_price, true_token_price=1):
        wallet = state.accounts['trader']
        return (Fraction(wallet.zec, ZAT) * true_zec_price
                + Fraction(wallet.tokens, MICRO) * true_token_price)

    def test_unchanged_oracle_cycles_lose_in_both_directions_and_when_split(self):
        policy = replace(Policy(), epoch_zec_cap=100 * ZAT,
                         epoch_token_cap=20_000 * MICRO)
        checks = 0
        for dollar_price in (20, 100, 103, 1000):
            for quantity in (101, MICRO + 1, 99 * MICRO):
                for pieces in (1, 3, 7):
                    for sides in (('buy', 'sell'), ('sell', 'buy')):
                        with self.subTest(price=dollar_price, quantity=quantity,
                                          pieces=pieces, sides=sides):
                            initial = self.initial(policy)
                            state = initial
                            prices = Prices(dollar_price * MICRO + 3, 1, 0)
                            amounts = [quantity // pieces + (i < quantity % pieces)
                                       for i in range(pieces)]
                            for side in sides:
                                for amount in amounts:
                                    state, _ = self.fill(state, policy, prices, side, amount)
                            self.assertEqual(state.accounts['trader'].tokens,
                                             initial.accounts['trader'].tokens)
                            self.assertLessEqual(state.accounts['trader'].zec,
                                                 initial.accounts['trader'].zec)
                            self.assertEqual(state.stable_supply, initial.stable_supply)
                            self.assertEqual(state.active_debt, initial.active_debt)
                            checks += 1
        self.assertEqual(checks, 72)

    def test_stale_conversion_after_zec_crash_transfers_true_value(self):
        policy = replace(Policy(), epoch_token_cap=10_000 * MICRO)
        initial = credit_funding(self.initial(policy), policy, 7 * ZAT, 0)
        state, fill = self.fill(initial, policy, Prices(100 * MICRO, 1, 0),
                                'sell', 10_000 * MICRO)
        self.assertEqual(fill['zec_amount'], 10_050_000_000)
        self.assertEqual(state.nsm_zec, 7 * ZAT)
        self.assertEqual(state.reserve_zec, initial.reserve_zec + fill['zec_amount'])
        self.assertEqual(self.trader_value(state, 20) - self.trader_value(initial, 20), 7990)
        self.assertEqual(self.public_value(initial, 20) - self.public_value(state, 20), 7990)
        # The stale mark books spread income while the assumed true mark loses.
        self.assertEqual(self.public_value(state, 100) - self.public_value(initial, 100), 50)

    def test_stale_conversion_after_zec_rise_overpays_sellers(self):
        policy = replace(Policy(), epoch_zec_cap=100 * ZAT)
        initial = self.initial(policy)
        state, fill = self.fill(initial, policy, Prices(100 * MICRO, 1, 0),
                                'buy', 10_000 * MICRO)
        self.assertEqual(fill['zec_amount'], 9_950_000_000)
        self.assertEqual(self.trader_value(state, 200) - self.trader_value(initial, 200), 9900)
        self.assertEqual(self.public_value(initial, 200) - self.public_value(state, 200), 9900)
        self.assertEqual(self.public_value(state, 100) - self.public_value(initial, 100), 50)

    def test_loss_making_cycle_can_capture_both_epoch_quotas(self):
        policy = Policy()
        initial = self.initial(policy)
        prices = Prices(100 * MICRO, 1, 0)
        state, _ = self.fill(initial, policy, prices, 'buy', 1000 * MICRO)
        state, _ = self.fill(state, policy, prices, 'sell', 1000 * MICRO)
        self.assertEqual(state.accounts['trader'].tokens, initial.accounts['trader'].tokens)
        self.assertEqual(initial.accounts['trader'].zec - state.accounts['trader'].zec, ZAT // 10)
        self.assertEqual(self.trader_value(initial, 100) - self.trader_value(state, 100), 10)
        self.assertEqual(state.epoch_zec_out, 995_000_000)
        self.assertEqual(state.epoch_token_out, policy.epoch_token_cap)
        for side in ('buy', 'sell'):
            with self.subTest(side=side):
                with self.assertRaisesRegex(ValueError, 'gross reserve budget'):
                    self.fill(state, policy, prices, side, 100 * MICRO)

    def test_lifetime_sale_cap_survives_new_epochs_and_funding(self):
        policy = replace(Policy(), epoch_token_cap=2000 * MICRO,
                         lifetime_token_cap=5000 * MICRO)
        initial = self.initial(policy)
        state = initial
        for epoch, quantity in enumerate((2000, 2000, 1000)):
            height = epoch * policy.epoch_blocks
            state, _ = self.fill(state, policy, Prices(100 * MICRO, epoch + 1, height),
                                 'sell', quantity * MICRO, height)
        self.assertEqual(state.lifetime_token_out, 5000 * MICRO)
        self.assertEqual(self.trader_value(state, 20) - self.trader_value(initial, 20), 3995)
        state = credit_funding(state, policy, 10 * ZAT, 30)
        balances = dict(state.accounts)
        counters = (state.reserve_zec, state.reserve_tokens, state.nsm_zec,
                    state.lifetime_token_out, state.epoch_token_out)
        with self.assertRaisesRegex(ValueError, 'gross reserve budget'):
            self.fill(state, policy, Prices(100 * MICRO, 4, 30), 'sell', MICRO, 30)
        self.assertEqual(dict(state.accounts), balances)
        self.assertEqual((state.reserve_zec, state.reserve_tokens, state.nsm_zec,
                          state.lifetime_token_out, state.epoch_token_out), counters)
        self.assertEqual(state.lifetime_token_out, 5000 * MICRO)


if __name__ == '__main__':
    unittest.main()
