"""Economic counterexamples, independent of the Rust consensus model."""

from fractions import Fraction
import unittest

from attacks import farm_profit, pool_displacement, proportional_payout, reserve_capacity


class RewardAttacks(unittest.TestCase):
    def test_retained_tokens_farm_both_epoch_rewards(self):
        profit = farm_profit(10_000, .01, .01, 2.5, .1, 1, 1)
        self.assertAlmostEqual(profit, 192.15068493150685)
        self.assertLess(farm_profit(10_000, .01, .01, 2.5, .1, 30, 1), 0)

    def test_break_even_duration_without_fixed_cost(self):
        self.assertAlmostEqual(farm_profit(10_000, .01, .01, 2.5, .1, 29.2, 0), 0)

    def test_wallet_cap_is_not_owner_cap(self):
        one = proportional_payout(Fraction(100), 1000, 100)
        ten = proportional_payout(Fraction(100), 1000, 1000)
        self.assertEqual(one, Fraction(100, 11))
        self.assertEqual(ten, 50)

    def test_thin_pool_price_trigger_cost(self):
        result = pool_displacement(1000, 1000, 1.02)
        self.assertAlmostEqual(result['quote_capital'], 9.95049383620783)
        self.assertAlmostEqual(result['marked_loss'], .0980368137635)
        new_x = 1000 - result['stable_received']
        new_y = 1000 + result['quote_capital']
        self.assertAlmostEqual(new_x * new_y, 1_000_000)
        self.assertAlmostEqual(new_y / new_x, 1.02)

    def test_bounty_capacity_is_not_purchase_capacity(self):
        self.assertEqual(reserve_capacity(1000, 20, .02), 1_000_000)
        self.assertEqual(reserve_capacity(1000, 10, .02), 500_000)
        self.assertEqual(reserve_capacity(1000, 20, 1), 20_000)


if __name__ == '__main__':
    unittest.main()
