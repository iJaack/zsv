"""Exact lifecycle transitions and retained-funding allowances."""

from dataclasses import FrozenInstanceError, replace
import unittest

from analysis.reserve import MICRO, U64_MAX, ZAT, Order, Policy, Prices, Wallet
from analysis.reserve_lifecycle import (Config, auction_price, bootstrap, credit_funding,
                                       execute, on_block, wind_down_buy)


class LifecycleTests(unittest.TestCase):
    def make(self, *, cash=100 * ZAT, inventory=10_000 * MICRO,
             external=20_000 * MICRO, bad_debt=0, config=None):
        state = bootstrap({'donor': Wallet(cash, inventory),
                           'market': Wallet(1000 * ZAT, external)},
                          'donor', cash, inventory, config=config or Config(),
                          bad_debt=bad_debt)
        return on_block(state, 0, Prices(100 * MICRO, 1, 0))

    def order(self, state, side='buy', tokens=100 * MICRO):
        return Order('market', side, tokens, state.ledger.accounts['market'].nonce,
                     state.latest_prices.zec_sequence, state.ledger.epoch,
                     state.ledger.height, 0 if side == 'buy' else U64_MAX)

    def trade(self, state, side='buy', tokens=100 * MICRO):
        return execute(state, self.order(state, side, tokens), state.ledger.height)[0]

    def fresh_through(self, state, height):
        for block in range(state.ledger.height + 1, height + 1):
            state = on_block(state, block, Prices(100 * MICRO,
                             state.latest_prices.zec_sequence + 1, block))
        return state

    def conserved(self, state):
        ledger = state.ledger
        self.assertEqual(ledger.reserve_tokens + sum(w.tokens for w in ledger.accounts.values()),
                         ledger.stable_supply)
        self.assertEqual(ledger.stable_supply, ledger.active_debt + ledger.bad_debt)
        self.assertEqual(ledger.reserve_zec + ledger.nsm_zec
                         + sum(w.zec for w in ledger.accounts.values()),
                         ledger.genesis_zec + ledger.funded_zec)

    def test_initial_cash_counts_and_over_target_credit_does_not_consume_budget(self):
        state = self.make()
        result = credit_funding(state, 7 * ZAT, 0)
        self.assertEqual(result.retained_funding_zec, 100 * ZAT)
        self.assertEqual(result.ledger.nsm_zec, 7 * ZAT)
        self.assertEqual(result.ledger.funded_zec, 7 * ZAT)
        self.conserved(result)

    def test_budget_caps_retained_credits_and_never_refunds_on_sale(self):
        config = replace(Config(), lifetime_retained_funding_cap=101 * ZAT)
        state = self.trade(self.make(config=config), tokens=1000 * MICRO)
        state = credit_funding(state, 10 * ZAT, 0)
        self.assertEqual(state.retained_funding_zec, 101 * ZAT)
        self.assertEqual(state.ledger.nsm_zec, 9 * ZAT)
        state = self.trade(state, 'sell', 1000 * MICRO)
        state = self.fresh_through(state, 10)
        state = self.trade(state, tokens=1000 * MICRO)
        state = credit_funding(state, 10 * ZAT, 10)
        self.assertEqual(state.retained_funding_zec, 101 * ZAT)
        self.assertEqual(state.ledger.nsm_zec, 19 * ZAT)
        self.conserved(state)

    def test_one_credit_per_prepared_height_including_zero(self):
        state = credit_funding(self.make(), 0, 0)
        with self.assertRaises(ValueError):
            credit_funding(state, ZAT, 0)
        with self.assertRaises(ValueError):
            credit_funding(state, ZAT, 1)

    def test_policy_is_embedded_and_immutable(self):
        state = self.make()
        with self.assertRaises(FrozenInstanceError):
            state.config.lifetime_retained_funding_cap = 0
        self.assertEqual(state.config.policy.lifetime_zec_cap, U64_MAX)
        with self.assertRaises(ValueError):
            self.make(config=Config(policy=Policy()))

    def test_twenty_epoch_cycle_no_longer_disables_later_trading(self):
        state = self.make()
        for epoch in range(20):
            height = epoch * 10
            state = self.fresh_through(state, height)
            state = self.trade(state, 'sell' if epoch < 10 else 'buy', 1000 * MICRO)
        self.assertEqual(state.ledger.reserve_zec, 101 * ZAT)
        self.assertEqual(state.retained_funding_zec, 100 * ZAT)
        state = self.fresh_through(state, 200)
        state = self.trade(state, 'buy', 100 * MICRO)
        self.assertGreater(state.ledger.lifetime_zec_out, 100 * ZAT)
        self.conserved(state)

    def test_epoch_limits_still_reject_atomically(self):
        state = self.trade(self.make(), 'sell', 1000 * MICRO)
        with self.assertRaises(ValueError):
            self.trade(state, 'sell', MICRO)
        self.assertEqual(state.ledger.epoch_token_out, 1000 * MICRO)

    def test_same_sequence_must_have_same_value_and_height(self):
        state = self.make()
        for prices in (Prices(101 * MICRO, 1, 0), Prices(100 * MICRO, 1, 1)):
            with self.assertRaises(ValueError):
                on_block(state, 1, prices)
        self.assertEqual(on_block(state, 1, state.latest_prices).latest_prices,
                         state.latest_prices)

    def test_stale_pause_can_resume_before_deadline(self):
        state = on_block(self.make(), 4)
        self.assertEqual(state.paused_since, 4)
        with self.assertRaises(ValueError):
            self.trade(state)
        state = on_block(state, 9, Prices(100 * MICRO, 2, 9))
        self.assertIsNone(state.paused_since)
        self.assertFalse(state.closed)
        self.conserved(self.trade(state))

    def test_late_observation_cannot_rescue_expired_outage(self):
        state = on_block(self.make(), 10, Prices(100 * MICRO, 2, 10))
        self.assertTrue(state.closed)
        self.assertEqual(state.shutdown.reason, 'oracle_outage')
        self.assertEqual(state.shutdown.height, 10)
        self.conserved(state)

    def test_no_initial_oracle_uses_activation_for_deadline(self):
        state = bootstrap({'donor': Wallet(ZAT, MICRO), 'holder': Wallet(0, MICRO)},
                          'donor', ZAT, MICRO)
        state = on_block(state, 6, Prices(100 * MICRO, 1, 6))
        self.assertTrue(state.closed)

    def test_sunset_closes_even_with_fresh_observation(self):
        config = replace(Config(), policy=replace(Config().policy, sunset_height=5))
        state = on_block(self.make(config=config), 5, Prices(100 * MICRO, 2, 5))
        self.assertTrue(state.closed)
        self.assertEqual(state.shutdown.reason, 'sunset')
        self.assertEqual(state.ledger.reserve_zec, 0)
        self.assertEqual(state.ledger.reserve_tokens, 10_000 * MICRO)
        self.assertEqual(state.ledger.nsm_zec, 100 * ZAT)
        self.conserved(state)

    def test_closed_reserve_routes_future_funding_to_nsm(self):
        state = on_block(self.make(), 10)
        receipt = state.shutdown
        state = on_block(state, 11, Prices(100 * MICRO, 2, 11))
        state = credit_funding(state, 3 * ZAT, 11)
        self.assertEqual(state.shutdown, receipt)
        self.assertEqual(state.retained_funding_zec, 100 * ZAT)
        self.assertEqual(state.ledger.nsm_zec, 103 * ZAT)
        self.assertEqual(state.ledger.reserve_zec, 0)
        with self.assertRaises(ValueError):
            self.trade(state)
        self.conserved(state)

    def test_shutdown_burns_bad_debt_and_reserves_remaining_inventory_for_sale(self):
        original = self.make(bad_debt=3000 * MICRO)
        state = on_block(original, 10)
        self.assertEqual(state.shutdown.bad_debt_burn, 3000 * MICRO)
        self.assertEqual(state.ledger.reserve_tokens, 7000 * MICRO)
        self.assertEqual(state.ledger.accounts, original.ledger.accounts)
        self.assertEqual(state.ledger.bad_debt, 0)
        self.assertEqual(state.ledger.active_debt, original.ledger.active_debt)
        self.assertEqual(state.ledger.stable_supply, original.ledger.stable_supply - 3000 * MICRO)
        self.assertEqual(state.ledger.accounts['market'].nonce,
                         original.ledger.accounts['market'].nonce)
        self.conserved(state)

    def test_insufficient_inventory_leaves_uncovered_bad_debt(self):
        state = on_block(self.make(bad_debt=15_000 * MICRO), 10)
        self.assertEqual(state.ledger.bad_debt, 5000 * MICRO)
        self.assertEqual(state.ledger.reserve_tokens, 0)
        self.conserved(state)

    def test_all_claims_in_reserve_does_not_forgive_active_debt(self):
        state = bootstrap({'donor': Wallet(ZAT, 10 * MICRO)},
                          'donor', ZAT, 10 * MICRO, bad_debt=3 * MICRO)
        state = on_block(state, 6)
        self.assertEqual(state.shutdown.bad_debt_burn, 3 * MICRO)
        self.assertEqual(state.ledger.reserve_tokens, 7 * MICRO)
        self.assertEqual(state.ledger.stable_supply, 7 * MICRO)
        self.assertEqual(state.ledger.active_debt, 7 * MICRO)
        self.conserved(state)

    def test_always_stale_new_reports_cannot_restart_outage_clock(self):
        state = self.make()
        for height in range(4, 11):
            state = on_block(state, height, Prices(100 * MICRO, height, height - 4))
            if height < 10:
                self.assertEqual(state.paused_since, 4)
        self.assertTrue(state.closed)
        self.assertEqual(state.shutdown.height, 10)

    def test_auction_schedule_and_persistent_positive_floor(self):
        config = replace(Config(), auction_duration_blocks=20)
        state = on_block(self.make(config=config), 10)
        self.assertEqual(auction_price(state, 10), ZAT)
        self.assertEqual(auction_price(state, 20), 50_000_001)
        self.assertEqual(auction_price(state, 30), 1)
        self.assertEqual(auction_price(state, 10_000), 1)

    def test_auction_sale_needs_no_oracle_and_preserves_debt(self):
        state = on_block(self.make(), 10)
        original = state
        state = on_block(state, 2010)
        state, fill = wind_down_buy(state, 'market', 100 * MICRO, 0, 100, 2010)
        self.assertEqual(fill['zec_amount'], 100)
        self.assertEqual(state.ledger.reserve_tokens, 9900 * MICRO)
        self.assertEqual(state.ledger.accounts['market'].tokens, 20_100 * MICRO)
        self.assertEqual(state.ledger.accounts['market'].nonce, 1)
        self.assertEqual(state.ledger.nsm_zec, original.ledger.nsm_zec + 100)
        self.assertEqual(state.ledger.active_debt, original.ledger.active_debt)
        self.assertEqual(state.ledger.stable_supply, original.ledger.stable_supply)
        self.conserved(state)
        state, _ = wind_down_buy(state, 'market', 9900 * MICRO, 1, 9900, 2010)
        self.assertEqual(state.ledger.reserve_tokens, 0)
        self.conserved(state)

    def test_auction_rejections_leave_inventory_and_wallets_unchanged(self):
        open_state = self.make()
        with self.assertRaises(ValueError):
            wind_down_buy(open_state, 'market', MICRO, 0, U64_MAX, 0)
        state = on_block(open_state, 10)
        for actor, tokens, nonce, limit, height in (
            ('market', MICRO, 0, 1, 10), ('unknown', MICRO, 0, U64_MAX, 10),
            ('market', 0, 0, U64_MAX, 10), ('market', 10_001 * MICRO, 0, U64_MAX, 10),
            ('market', MICRO, 1, U64_MAX, 10), ('market', MICRO, 0, U64_MAX, 11),
        ):
            with self.assertRaises(ValueError):
                wind_down_buy(state, actor, tokens, nonce, limit, height)
        self.assertEqual(state.ledger.reserve_tokens, 10_000 * MICRO)
        self.assertEqual(state.ledger.accounts, open_state.ledger.accounts)

    def test_splitting_auction_fills_cannot_reduce_payment(self):
        state = on_block(self.make(), 10)
        state = on_block(state, 2010)
        whole, fill = wind_down_buy(state, 'market', MICRO + 1, 0, U64_MAX, 2010)
        split, a = wind_down_buy(state, 'market', MICRO, 0, U64_MAX, 2010)
        split, b = wind_down_buy(split, 'market', 1, 1, U64_MAX, 2010)
        self.assertGreaterEqual(a['zec_amount'] + b['zec_amount'], fill['zec_amount'])
        self.assertEqual(split.ledger.reserve_tokens, whole.ledger.reserve_tokens)
        self.assertGreaterEqual(split.ledger.nsm_zec, whole.ledger.nsm_zec)
        self.conserved(whole)
        self.conserved(split)

    def test_invalid_bootstrap_and_metadata_fail(self):
        with self.assertRaises(ValueError):
            self.make(config=replace(Config(), lifetime_retained_funding_cap=ZAT))
        with self.assertRaises(ValueError):
            self.make(config=replace(Config(), max_accounts=1))
        state = self.make()
        for height, prices in ((-1, None), (1, Prices(0, 2, 1)),
                               (1, Prices(MICRO, 2, 2)), (1, Prices(MICRO, 0, 1))):
            with self.assertRaises(ValueError):
                on_block(state, height, prices)
        with self.assertRaises(ValueError):
            credit_funding(state, U64_MAX, 0)
        self.conserved(state)

    def test_monotonic_block_context_and_preparation_required(self):
        state = on_block(self.make(), 1)
        with self.assertRaises(ValueError):
            on_block(state, 0)
        with self.assertRaises(ValueError):
            execute(state, self.order(state), 2)
        unprepared = bootstrap({'donor': Wallet(ZAT, MICRO)}, 'donor', ZAT, MICRO)
        with self.assertRaises(ValueError):
            credit_funding(unprepared, ZAT, 0)


if __name__ == '__main__':
    unittest.main()
