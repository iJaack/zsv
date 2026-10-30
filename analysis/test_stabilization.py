"""Analytical checks and counterexamples for the hypothetical incentive model."""

import math
import unittest
from dataclasses import replace
from fractions import Fraction

from analysis.stabilization import (
    Scenario,
    ideal_errors,
    signed_budget_difference,
    simulate,
    split_cut,
    tapered_share,
)


class FundingTests(unittest.TestCase):
    def test_integer_split_conserves_the_reference_allocation(self):
        for month in (0, 1, 12, 24, 48, 72, 120):
            for subsidy in (0, 1, 101, 156_250_000, 180_000_000_000):
                for alpha in (Fraction(0), Fraction(1, 10), Fraction(1)):
                    with self.subTest(month=month, subsidy=subsidy, alpha=alpha):
                        result = split_cut(subsidy, Fraction(2, 25), tapered_share(month), alpha)
                        self.assertEqual(
                            result["grants"] + result["incentives"] + result["nsm"],
                            result["reference"],
                        )
                        self.assertGreaterEqual(result["nsm"], 0)
                        if alpha == Fraction(1, 10):
                            self.assertLessEqual(result["incentives"], subsidy // 125)

    def test_month_zero_has_no_incentive_credit(self):
        self.assertEqual(tapered_share(0), Fraction(2, 25))
        result = split_cut(180_000_000_000, Fraction(2, 25), tapered_share(0), Fraction(1, 10))
        self.assertEqual(result["incentives"], 0)

    def test_month_twelve_matches_exact_fraction(self):
        share = tapered_share(12)
        self.assertEqual(share, Fraction(2, 25) * Fraction(97, 100) ** 12)
        fraction_of_subsidy = Fraction(1, 10) * (Fraction(2, 25) - share)
        self.assertAlmostEqual(float(fraction_of_subsidy), 0.002449261112036496, places=15)

    def test_bad_allocations_are_rejected(self):
        for subsidy, reference, proposed, alpha in (
            (-1, Fraction(1), Fraction(0), Fraction(1)),
            (1, Fraction(-1), Fraction(0), Fraction(1)),
            (1, Fraction(1), Fraction(0), Fraction(2)),
            (1, Fraction(0), Fraction(1, 10), Fraction(1)),
            (1, 0.08, Fraction(0), Fraction(1)),
        ):
            with self.subTest(subsidy=subsidy, reference=reference, proposed=proposed, alpha=alpha):
                with self.assertRaises(ValueError):
                    split_cut(subsidy, reference, proposed, alpha)
        with self.assertRaises(ValueError):
            tapered_share(-1)

    def test_current_law_expiry_cannot_be_dropped_from_the_comparison(self):
        subsidies = [100_000, 50_000, 50_000]
        reference = [Fraction(2, 25), Fraction(0), Fraction(0)]
        proposed = [Fraction(1, 25)] * 3
        self.assertEqual(signed_budget_difference(subsidies, reference, proposed), 0)
        self.assertEqual(signed_budget_difference(subsidies, [Fraction(2, 25)] * 3, proposed), 8_000)

    def test_signed_budget_comparison_includes_net_spending(self):
        self.assertEqual(signed_budget_difference([100], [Fraction(0)], [Fraction(1, 10)]), -10)
        with self.assertRaises(ValueError):
            signed_budget_difference([100], [], [])


class IdealControlTests(unittest.TestCase):
    def test_no_delay_geometric_solution(self):
        errors = ideal_errors(0.5, 0, 0.05, 20)
        for epoch, error in enumerate(errors):
            self.assertAlmostEqual(error, 0.05 * 0.5**epoch, places=15)

    def test_no_delay_stability_boundary(self):
        self.assertLess(abs(ideal_errors(1.5, 0, 0.05, 100)[-1]), 1e-20)
        self.assertAlmostEqual(abs(ideal_errors(2, 0, 0.05, 100)[-1]), 0.05)
        self.assertGreater(abs(ideal_errors(2.1, 0, 0.05, 20)[-1]), 0.05)

    def test_delay_one_has_a_six_epoch_cycle_at_gain_one(self):
        errors = ideal_errors(1, 1, 0.05, 18)
        for epoch in range(13):
            self.assertAlmostEqual(errors[epoch], errors[epoch + 6], places=15)
        self.assertAlmostEqual(max(abs(error) for error in errors), 0.05)

    def test_delay_one_converges_below_boundary_and_grows_above_it(self):
        self.assertLess(abs(ideal_errors(0.5, 1, 0.05, 100)[-1]), 1e-15)
        errors = ideal_errors(1.1, 1, 0.05, 100)
        self.assertGreater(max(abs(error) for error in errors[-10:]), 1)

    def test_overshoot_increases_total_action_cost(self):
        depth, initial_error = 100_000, 0.05
        for gain in (0.5, 1, 1.5, 1.9):
            errors = ideal_errors(gain, 0, initial_error, 1_000)
            action = sum(abs(gain * depth * error) for error in errors[:-1])
            expected = depth * initial_error * (1 if gain <= 1 else gain / (2 - gain))
            self.assertAlmostEqual(action, expected, places=6)

    def test_invalid_parameters_are_rejected(self):
        for args in ((math.nan, 0, 0.05, 2), (-1, 0, 0.05, 2), (1, 2, 0.05, 2), (1, 0, math.inf, 2), (1, 0, 0.05, -1)):
            with self.subTest(args=args):
                with self.assertRaises(ValueError):
                    ideal_errors(*args)


class BudgetedControlTests(unittest.TestCase):
    def setUp(self):
        self.base = Scenario(name="base")

    def test_sufficient_budget_recovers_a_single_shock(self):
        rows = simulate(self.base)
        self.assertEqual(len(rows), 31)
        self.assertLess(abs(rows[-1]["error"]), 1e-9)
        self.assertAlmostEqual(sum(row["payout_usd"] for row in rows), 50, places=6)
        self.assertAlmostEqual(rows[-1]["reserve_zec"], 0.5, places=8)

    def test_insufficient_budget_leaves_a_residual_depeg(self):
        rows = simulate(replace(self.base, reserve_zec=0.1))
        self.assertAlmostEqual(rows[-1]["price"], 1.04)
        self.assertAlmostEqual(rows[-1]["reserve_zec"], 0)
        self.assertAlmostEqual(sum(row["payout_usd"] for row in rows), 10)
        self.assertTrue(any(row["capacity_bound"] for row in rows))

    def test_no_response_spends_nothing_and_does_not_change_price(self):
        rows = simulate(replace(self.base, response=0))
        self.assertEqual(rows[0]["price"], rows[-1]["price"])
        self.assertEqual(sum(row["payout_usd"] for row in rows), 0)
        self.assertEqual(rows[-1]["reserve_zec"], 1)

    def test_collateral_headroom_binds_issuance(self):
        rows = simulate(replace(self.base, collateral_zec=2_000))
        self.assertEqual(rows[0]["collateral_headroom"], 0)
        self.assertEqual(rows[-1]["supply"], 80_000)
        self.assertEqual(rows[-1]["price"], 1.05)

    def test_bad_debt_freezes_new_borrowing_despite_available_budget(self):
        rows = simulate(replace(self.base, bad_debt=True))
        self.assertEqual(rows[-1]["supply"], 80_000)
        self.assertEqual(sum(row["payout_usd"] for row in rows), 0)

    def test_burn_reduces_debt_and_supply_by_the_same_quantity(self):
        rows = simulate(replace(self.base, target_demand=75_000))
        self.assertLess(abs(rows[-1]["error"]), 1e-9)
        self.assertAlmostEqual(rows[-1]["debt"], 75_000, places=5)
        for row in rows:
            self.assertEqual(row["debt"], row["supply"])
            self.assertGreaterEqual(row["reserve_zec"], 0)
            self.assertGreaterEqual(row["reserve_after_zec"], 0)

    def test_funding_is_available_before_the_first_action(self):
        rows = simulate(replace(self.base, epochs=1, reserve_zec=0, funding_zec=0.1))
        self.assertAlmostEqual(rows[0]["action"], 1_000)
        self.assertEqual(rows[0]["reserve_zec"], 0)
        self.assertEqual(rows[0]["funding_zec"], 0.1)
        self.assertEqual(rows[-1]["funding_zec"], 0)
        self.assertEqual(rows[-1]["payout_usd"], 0)

    def test_price_drop_reduces_fixed_dollar_bounty_capacity(self):
        high = simulate(replace(self.base, epochs=1, gain=1, reserve_zec=0.1))
        low = simulate(replace(self.base, epochs=1, gain=1, reserve_zec=0.1, zec_usd=50, collateral_zec=5_000))
        self.assertAlmostEqual(low[0]["action"], high[0]["action"] / 2)

    def test_same_collateral_loses_all_borrowing_headroom_after_price_drop(self):
        rows = simulate(replace(self.base, zec_usd=50))
        self.assertEqual(rows[0]["collateral_headroom"], 0)
        self.assertEqual(rows[-1]["price"], 1.05)

    def test_borrowing_freeze_still_allows_repayment(self):
        rows = simulate(replace(self.base, bad_debt=True, target_demand=75_000))
        self.assertLess(rows[-1]["debt"], rows[0]["debt"])
        self.assertLess(abs(rows[-1]["error"]), 1e-9)

    def test_each_epoch_conserves_the_reward_reserve(self):
        rows = simulate(replace(self.base, reserve_zec=0.03, funding_zec=0.01, delay=1))
        for row, following in zip(rows, rows[1:]):
            self.assertAlmostEqual(
                row["reserve_zec"] + row["funding_zec"],
                row["reserve_after_zec"] + row["payout_usd"] / self.base.zec_usd,
            )
            self.assertEqual(following["reserve_zec"], row["reserve_after_zec"])

    def test_persistent_demand_shocks_can_deplete_reserve(self):
        rows = simulate(replace(self.base, target_demand=80_000, epochs=30, demand_step_per_epoch=-1_000))
        self.assertAlmostEqual(rows[-1]["reserve_zec"], 0)
        self.assertLess(rows[-1]["price"], 0.9)

    def test_debt_ceiling_is_enforced(self):
        rows = simulate(replace(self.base, debt_ceiling=81_000))
        self.assertEqual(rows[-1]["debt"], 81_000)
        self.assertAlmostEqual(rows[-1]["price"], 1.04)

    def test_invalid_scenarios_are_rejected(self):
        for change in (
            {"reward_usd": 0}, {"zec_usd": 0}, {"depth": 0},
            {"reserve_zec": -1}, {"response": 1.1}, {"funding_zec": -1},
            {"initial_supply": 100_001}, {"delay": 2}, {"gain": math.inf},
        ):
            with self.subTest(change=change):
                with self.assertRaises(ValueError):
                    simulate(replace(self.base, **change))

    def test_negative_price_is_a_model_domain_failure(self):
        with self.assertRaises(ValueError):
            simulate(replace(self.base, target_demand=0, depth=1))


if __name__ == "__main__":
    unittest.main()
