"""Independent outage timing and auction-price counterexamples."""

from dataclasses import replace
from fractions import Fraction
import unittest

from analysis.reserve import MICRO, U64_MAX, ZAT, Prices, Wallet
from analysis.reserve_lifecycle import (Config, auction_price, bootstrap,
                                       on_block, wind_down_buy)


class LifecycleReviewTests(unittest.TestCase):
    def fixture(self, config=Config()):
        state = bootstrap({'donor': Wallet(100 * ZAT, 10_000 * MICRO),
                           'market': Wallet(1000 * ZAT, 80_000 * MICRO)},
                          'donor', 100 * ZAT, 10_000 * MICRO, config)
        return on_block(state, 0, Prices(100 * MICRO, 1, 0))

    def test_stale_report_trickle_keeps_original_deadline_across_parameters(self):
        for age in (0, 1, 3, 7):
            for grace in (1, 2, 6):
                with self.subTest(age=age, grace=grace):
                    config = replace(Config(), oracle_grace_blocks=grace,
                                     policy=replace(Config().policy, max_age=age))
                    state = self.fixture(config)
                    deadline = age + 1 + grace
                    for height in range(1, deadline + 1):
                        observation_height = max(0, height - age - 1)
                        state = on_block(state, height,
                                         Prices(100 * MICRO, height + 1, observation_height))
                        self.assertEqual(state.closed, height == deadline)
                    self.assertEqual(state.shutdown.height, deadline)
                    self.assertEqual(state.shutdown.reason, 'oracle_outage')

    def test_fresh_backdated_report_uses_observation_age_after_resume(self):
        state = on_block(self.fixture(), 4)
        state = on_block(state, 9, Prices(100 * MICRO, 2, 6))
        self.assertIsNone(state.paused_since)
        self.assertFalse(state.closed)
        state = on_block(state, 10)
        self.assertEqual(state.paused_since, 10)
        state = on_block(state, 15)
        self.assertFalse(state.closed)
        state = on_block(state, 16, Prices(100 * MICRO, 3, 16))
        self.assertTrue(state.closed)
        self.assertEqual(state.shutdown.height, 16)

    def test_auction_granularity_can_transfer_value_even_with_competing_buyers(self):
        state = on_block(self.fixture(), 10)
        first_height = state.shutdown.height + 1981
        self.assertEqual(auction_price(state, first_height - 1), 1_000_001)
        self.assertEqual(auction_price(state, first_height), 950_001)
        state = on_block(state, first_height)
        original = state
        state, receipt = wind_down_buy(state, 'market', 10_000 * MICRO, 0,
                                       U64_MAX, first_height)
        paid_usd = Fraction(receipt['zec_amount'] * 100, ZAT)
        self.assertEqual(paid_usd, Fraction(950_001, 100))
        self.assertEqual(10_000 - paid_usd, Fraction(49_999, 100))
        self.assertEqual(state.ledger.active_debt, original.ledger.active_debt)
        self.assertEqual(state.ledger.stable_supply, original.ledger.stable_supply)
        self.assertEqual(state.ledger.reserve_tokens, 0)
        self.assertEqual(state.ledger.nsm_zec - original.ledger.nsm_zec,
                         receipt['zec_amount'])


if __name__ == '__main__':
    unittest.main()
