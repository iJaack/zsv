"""Check intervention outcomes against independent inventory/capital bounds."""

from dataclasses import replace
from fractions import Fraction
import unittest

from analysis.reserve import MICRO, ZAT, Policy, Prices, Wallet, bootstrap
from analysis.reserve_study import MarketCase, fill_largest, hostile_fill, simulate_market, summarize


class InventoryMarketTests(unittest.TestCase):
    def test_purchase_reaches_bid_and_pays_full_cost(self):
        rows = simulate_market(MarketCase('base'))
        result = summarize(rows)
        self.assertEqual(result['final_price'], .995)
        self.assertEqual(result['purchased_tokens'], 4500)
        fills = sum(row['filled_tokens'] > 0 for row in rows)
        self.assertLessEqual(result['zec_paid'], 44.775)
        self.assertGreater(result['zec_paid'], 44.775 - fills / ZAT)
        self.assertEqual(result['final_inventory_tokens'], 14_500)

    def test_sale_reaches_ask_using_existing_inventory(self):
        rows = simulate_market(MarketCase('sale', demand=85_000))
        result = summarize(rows)
        self.assertEqual(result['final_price'], 1.005)
        self.assertEqual(result['sold_tokens'], 4500)
        self.assertAlmostEqual(result['zec_received'], 45.225)
        self.assertEqual(result['final_inventory_tokens'], 5500)
        self.assertTrue(all(row['total_supply_tokens'] == 90_000 for row in rows))

    def test_empty_inventory_cannot_supply_above_peg_demand(self):
        result = summarize(simulate_market(MarketCase('empty', inventory=0, demand=85_000)))
        self.assertEqual(result['final_price'], 1.05)
        self.assertEqual(result['sold_tokens'], 0)

    def test_demand_beyond_existing_supply_leaves_a_depeg(self):
        result = summarize(simulate_market(MarketCase('excess', demand=100_000)))
        self.assertEqual(result['final_price'], 1.1)
        self.assertEqual(result['final_inventory_tokens'], 0)

    def test_transaction_cost_hurdle_widens_effective_band(self):
        result = summarize(simulate_market(MarketCase('cost', hurdle=Fraction(1, 100))))
        self.assertEqual(result['final_price'], .985)
        self.assertEqual(result['purchased_tokens'], 3500)

    def test_largest_fill_handles_lower_dust_boundary(self):
        policy = Policy()
        state = bootstrap({'donor': Wallet(ZAT, 0), 'market': Wallet(0, 100 * MICRO)},
                          'donor', ZAT, 0, policy)
        unchanged, receipt = fill_largest(state, policy, Prices(100 * MICRO, 1, 0), 'buy', 1, 0)
        self.assertIs(unchanged, state)
        self.assertIsNone(receipt)
        changed, receipt = fill_largest(state, policy, Prices(100 * MICRO, 1, 0), 'buy', 2, 0)
        self.assertEqual(receipt['zec_amount'], 1)
        self.assertEqual(changed.reserve_tokens, 2)

    def test_hostile_fill_measures_true_prices_separately(self):
        self.assertEqual(hostile_fill('sell', 20)['trader_gain_usd'], 7990)
        self.assertEqual(hostile_fill('buy', 200)['trader_gain_usd'], 9900)
        self.assertEqual(hostile_fill('buy', 100, Fraction(1, 2))['trader_gain_usd'], 4950)

    def test_expiry_and_no_participants_stop_intervention(self):
        base = MarketCase('base')
        expired = summarize(simulate_market(replace(base, sunset=20)))
        inactive = summarize(simulate_market(replace(base, response=Fraction(0))))
        self.assertLess(expired['final_price'], .995)
        self.assertEqual(inactive['final_price'], .95)


if __name__ == '__main__':
    unittest.main()
