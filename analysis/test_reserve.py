"""Exact transfers and hostile orders for the bounded research reserve."""

from dataclasses import replace
from fractions import Fraction
import unittest

from analysis.reserve import (MICRO, U64_MAX, ZAT, Order, Policy, Prices,
                              Wallet, bootstrap, credit_funding, execute)


class ReserveTests(unittest.TestCase):
    def setUp(self):
        self.policy = Policy()
        self.state = bootstrap({'donor': Wallet(1000 * ZAT, 20_000 * MICRO),
                                'trader': Wallet(1000 * ZAT, 20_000 * MICRO)},
                               'donor', 100 * ZAT, 10_000 * MICRO)
        self.prices = Prices(100 * MICRO, 1, 0)

    def order(self, state=None, prices=None, side='buy', tokens=100 * MICRO, height=0):
        state = state or self.state
        prices = prices or self.prices
        return Order('trader', side, tokens, state.accounts['trader'].nonce,
                     prices.zec_sequence,
                     height // self.policy.epoch_blocks, height,
                     0 if side == 'buy' else U64_MAX)

    def reject(self, state=None, prices=None, policy=None, order=None, height=0):
        state = state or self.state
        original = (state, dict(state.accounts))
        with self.assertRaises(ValueError):
            execute(state, policy or self.policy, prices or self.prices,
                    order or self.order(state, prices, height=height), height)
        self.assertEqual((state, dict(state.accounts)), original)

    def conserved(self, state):
        self.assertEqual(sum(w.tokens for w in state.accounts.values()) + state.reserve_tokens,
                         state.stable_supply)
        self.assertEqual(state.stable_supply, state.active_debt + state.bad_debt)
        self.assertEqual(sum(w.zec for w in state.accounts.values()) + state.reserve_zec + state.nsm_zec,
                         state.genesis_zec + state.funded_zec)

    def test_bootstrap_inventory_is_debited_from_named_donor(self):
        self.assertEqual(self.state.stable_supply, 40_000 * MICRO)
        self.assertEqual(self.state.accounts['donor'], Wallet(900 * ZAT, 10_000 * MICRO))
        self.conserved(self.state)
        with self.assertRaises(ValueError):
            bootstrap({'donor': Wallet(0, 1)}, 'donor', 0, 2)

    def test_buy_transfers_both_assets_and_retains_tokens(self):
        state, fill = execute(self.state, self.policy, self.prices, self.order(), 0)
        self.assertEqual(fill['zec_amount'], 99_500_000)
        self.assertEqual(state.reserve_tokens, 10_100 * MICRO)
        self.assertEqual(state.accounts['trader'].tokens, 19_900 * MICRO)
        self.assertEqual(state.accounts['trader'].nonce, 1)
        self.assertEqual(state.epoch_zec_out, fill['zec_amount'])
        self.assertEqual(self.state.reserve_tokens, 10_000 * MICRO)
        self.conserved(state)

    def test_sell_transfers_existing_inventory_and_charges_ceiling(self):
        prices = self.prices
        order = self.order(prices=prices, side='sell')
        state, fill = execute(self.state, self.policy, prices, order, 0)
        self.assertEqual(fill['zec_amount'], 100_500_000)
        self.assertEqual(state.reserve_tokens, 9900 * MICRO)
        self.assertEqual(state.epoch_token_out, 100 * MICRO)
        self.conserved(state)

    def test_rounding_is_favorable_to_reserve_at_fractional_boundaries(self):
        policy = self.policy
        for side, quote in [('buy', 995_000), ('sell', 1_005_000)]:
            for amount in (10, 101, MICRO + 1):
                prices = replace(self.prices, zec_usd=103 * MICRO + 3)
                state, fill = execute(self.state, policy, prices,
                                      self.order(prices=prices, side=side, tokens=amount), 0)
                exact = Fraction(amount * quote * ZAT, MICRO * prices.zec_usd)
                if side == 'buy':
                    self.assertLessEqual(fill['zec_amount'], exact)
                    self.assertLess(exact - fill['zec_amount'], 1)
                else:
                    self.assertGreaterEqual(fill['zec_amount'], exact)
                    self.assertLess(fill['zec_amount'] - exact, 1)
                self.conserved(state)

    def test_zero_output_dust_rejected(self):
        prices = replace(self.prices, zec_usd=U64_MAX)
        self.reject(prices=prices, order=self.order(prices=prices, tokens=1))

    def test_each_order_binding_rejects_atomically(self):
        order = self.order()
        changes = ({'actor': 'unknown'}, {'side': 'mint'}, {'tokens': 0},
                   {'nonce': 1}, {'zec_sequence': 2},
                   {'epoch': 1}, {'limit_zec': ZAT})
        for change in changes:
            with self.subTest(change=change):
                self.reject(order=replace(order, **change))

    def test_sell_maximum_input_and_expired_order_reject(self):
        prices = self.prices
        self.reject(prices=prices, order=replace(self.order(prices=prices, side='sell'), limit_zec=1))
        self.reject(order=self.order(), height=1)

    def test_replay_rejected_after_successful_fill(self):
        order = self.order()
        state, _ = execute(self.state, self.policy, self.prices, order, 0)
        self.reject(state=state, order=order)

    def test_oracle_must_be_fresh_and_not_future_dated(self):
        for observation, height in [(4, 0), (0, 4)]:
            prices = replace(self.prices, zec_height=observation)
            self.reject(prices=prices, order=self.order(prices=prices, height=height), height=height)

    def test_feed_sequences_cannot_roll_back(self):
        prices = replace(self.prices, zec_sequence=2)
        state, _ = execute(self.state, self.policy, prices, self.order(prices=prices), 0)
        self.reject(state=state, order=self.order(state))

    def test_zero_and_out_of_range_prices_are_rejected(self):
        for change in ({'zec_usd': 0}, {'zec_usd': U64_MAX + 1},
                       {'zec_sequence': -1}, {'zec_height': True}):
            prices = replace(self.prices, **change)
            self.reject(prices=prices, order=self.order(prices=prices))

    def test_epoch_zec_budget_is_gross_and_shared_across_fills(self):
        policy = replace(self.policy, epoch_zec_cap=ZAT)
        state, _ = execute(self.state, policy, self.prices, self.order(), 0)
        self.reject(state=state, policy=policy, order=self.order(state))
        state = credit_funding(state, policy, 10 * ZAT, 0)
        self.reject(state=state, policy=policy, order=self.order(state))

    def test_new_epoch_resets_only_epoch_budgets(self):
        policy = replace(self.policy, epoch_zec_cap=ZAT)
        state, _ = execute(self.state, policy, self.prices, self.order(), 0)
        prices = replace(self.prices, zec_height=10, zec_sequence=2)
        order = self.order(state, prices, height=10)
        advanced, _ = execute(state, policy, prices, order, 10)
        self.assertEqual(advanced.epoch_zec_out, 99_500_000)
        self.assertEqual(advanced.lifetime_zec_out, 199_000_000)
        self.reject(state=advanced, prices=prices, order=self.order(advanced, prices, height=0), height=0)

    def test_lifetime_zec_budget_survives_epoch_reset(self):
        policy = replace(self.policy, lifetime_zec_cap=ZAT)
        state, _ = execute(self.state, policy, self.prices, self.order(), 0)
        prices = replace(self.prices, zec_height=10)
        self.reject(state=state, policy=policy, prices=prices,
                    order=self.order(state, prices, height=10), height=10)

    def test_epoch_and_lifetime_token_sale_caps(self):
        prices = self.prices
        for field in ('epoch_token_cap', 'lifetime_token_cap'):
            policy = replace(self.policy, **{field: 150 * MICRO})
            state, _ = execute(self.state, policy, prices, self.order(prices=prices, side='sell'), 0)
            self.reject(state=state, policy=policy, prices=prices,
                        order=self.order(state, prices, side='sell'))

    def test_replenishing_token_inventory_does_not_reset_sale_meter(self):
        policy = replace(self.policy, epoch_token_cap=150 * MICRO)
        state, _ = execute(self.state, policy, self.prices, self.order(side='sell'), 0)
        state, _ = execute(state, policy, self.prices, self.order(state), 0)
        self.assertEqual(state.reserve_tokens, self.state.reserve_tokens)
        self.reject(state=state, policy=policy, order=self.order(state, side='sell'))

    def test_lifetime_token_sale_cap_survives_epoch_reset(self):
        policy = replace(self.policy, lifetime_token_cap=150 * MICRO)
        state, _ = execute(self.state, policy, self.prices, self.order(side='sell'), 0)
        prices = replace(self.prices, zec_height=10, zec_sequence=2)
        self.reject(state=state, policy=policy, prices=prices,
                    order=self.order(state, prices, side='sell', height=10), height=10)

    def test_reserve_floor_inventory_limit_and_asset_availability(self):
        for change in ({'min_reserve_zec': 100 * ZAT}, {'max_reserve_tokens': 10_000 * MICRO}):
            self.reject(policy=replace(self.policy, **change))
        prices = self.prices
        policy = replace(self.policy, epoch_token_cap=20_000 * MICRO)
        self.reject(prices=prices, policy=policy,
                    order=self.order(prices=prices, side='sell', tokens=11_000 * MICRO))

    def test_activation_sunset_and_monotonic_heights(self):
        self.reject(policy=replace(self.policy, activation_height=1))
        policy = replace(self.policy, sunset_height=1)
        prices = replace(self.prices, zec_height=1)
        self.reject(policy=policy, prices=prices, order=self.order(prices=prices, height=1), height=1)

    def test_funding_overflow_is_recorded_and_conserved(self):
        state = credit_funding(self.state, self.policy, 7 * ZAT, 0)
        self.assertEqual(state.reserve_zec, 100 * ZAT)
        self.assertEqual(state.nsm_zec, 7 * ZAT)
        self.assertEqual(state.funded_zec, 7 * ZAT)
        self.conserved(state)
        self.assertEqual(self.state.nsm_zec, 0)

    def test_sale_proceeds_above_funding_target_are_retained(self):
        state, fill = execute(self.state, self.policy, self.prices,
                              self.order(side='sell'), 0)
        self.assertEqual(state.reserve_zec, self.policy.funding_target_zec + fill['zec_amount'])
        self.assertEqual(state.nsm_zec, 0)
        self.assertEqual(fill['nsm_overflow'], 0)
        state = credit_funding(state, self.policy, ZAT, 0)
        self.assertEqual(state.reserve_zec, self.policy.funding_target_zec + fill['zec_amount'])
        self.assertEqual(state.nsm_zec, ZAT)
        self.conserved(state)

    def test_buy_then_sell_cycle_loses_to_spread_and_splitting(self):
        for pieces in (1, 3, 7):
            state = self.state
            tokens = 99 * MICRO
            for i in range(pieces):
                q = tokens // pieces + (i < tokens % pieces)
                state, _ = execute(state, self.policy, self.prices,
                                   self.order(state, tokens=q), 0)
            prices = replace(self.prices, zec_sequence=2)
            for i in range(pieces):
                q = tokens // pieces + (i < tokens % pieces)
                state, _ = execute(state, self.policy, prices,
                                   self.order(state, prices, side='sell', tokens=q), 0)
            self.assertEqual(state.accounts['trader'].tokens, self.state.accounts['trader'].tokens)
            self.assertLessEqual(state.accounts['trader'].zec,
                                 self.state.accounts['trader'].zec - 990_000)
            self.conserved(state)

    def test_sell_then_buy_cycle_cannot_profit(self):
        state, _ = execute(self.state, self.policy, self.prices, self.order(side='sell'), 0)
        state, _ = execute(state, self.policy, self.prices, self.order(state), 0)
        self.assertEqual(state.accounts['trader'].tokens, self.state.accounts['trader'].tokens)
        self.assertEqual(state.accounts['trader'].zec, self.state.accounts['trader'].zec - 1_000_000)
        self.conserved(state)

    def test_repeated_sales_and_buybacks_retain_working_capital(self):
        state = self.state
        for i in range(20):
            height = i * self.policy.epoch_blocks
            prices = replace(self.prices, zec_sequence=i + 1, zec_height=height)
            side = 'sell' if i < 10 else 'buy'
            order = self.order(state, prices, side=side, tokens=1000 * MICRO, height=height)
            state, _ = execute(state, self.policy, prices, order, height)
        self.assertEqual(state.reserve_tokens, self.state.reserve_tokens)
        self.assertEqual(state.reserve_zec, 101 * ZAT)
        self.assertEqual(state.nsm_zec, 0)
        self.assertEqual(state.accounts['trader'].zec, self.state.accounts['trader'].zec - ZAT)
        self.conserved(state)

    def test_nonce_overflow_rejects_atomically(self):
        accounts = {'donor': Wallet(200 * ZAT, 20_000 * MICRO),
                    'trader': Wallet(ZAT, MICRO, U64_MAX)}
        state = bootstrap(accounts, 'donor', 100 * ZAT, 10_000 * MICRO)
        self.reject(state=state, order=self.order(state, tokens=MICRO))

    def test_u64_balance_and_order_bounds(self):
        for amount in (-1, U64_MAX + 1, True):
            self.reject(order=replace(self.order(), tokens=amount))
        with self.assertRaises(ValueError):
            bootstrap({'donor': Wallet(U64_MAX + 1, 0)}, 'donor', 0, 0)
        with self.assertRaises(ValueError):
            credit_funding(self.state, self.policy, U64_MAX, 0)


if __name__ == '__main__':
    unittest.main()
