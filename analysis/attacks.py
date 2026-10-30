"""Illustrative economics of proposed incentives; no deployed mechanism."""

import math


def farm_profit(quantity, mint_reward, burn_reward, collateral_per_token,
                annual_cost, holding_days, fixed_cost):
    """USD profit from minting, retaining and later burning the same units."""
    values = (quantity, mint_reward, burn_reward, collateral_per_token,
              annual_cost, holding_days, fixed_cost)
    if any(not math.isfinite(value) or value < 0 for value in values):
        raise ValueError('nonnegative finite inputs required')
    financing = quantity * collateral_per_token * annual_cost * holding_days / 365
    return quantity * (mint_reward + burn_reward) - financing - fixed_cost


def proportional_payout(pool, honest_weight, farmer_weight):
    """Caller supplies eligible weight after any per-wallet cap."""
    if min(pool, honest_weight, farmer_weight) < 0 or honest_weight + farmer_weight <= 0:
        raise ValueError('nonnegative weights and positive total required')
    return pool * farmer_weight / (honest_weight + farmer_weight)


def pool_displacement(stable_reserve, quote_reserve, price_multiple):
    """Upward constant-product swap, excluding fees and intervening trades."""
    if (not all(math.isfinite(x) for x in (stable_reserve, quote_reserve, price_multiple))
            or min(stable_reserve, quote_reserve) <= 0 or price_multiple < 1):
        raise ValueError('positive reserves and finite price multiple >= 1 required')
    root = math.sqrt(price_multiple)
    capital = quote_reserve * (root - 1)
    received = stable_reserve * (1 - 1 / root)
    return {'quote_capital': capital, 'stable_received': received,
            'marked_loss': capital - (quote_reserve / stable_reserve) * received}


def reserve_capacity(reserve_zec, zec_usd, spend_usd_per_token):
    """Tokens affordable for the specified actual expenditure per token."""
    if (not all(math.isfinite(x) for x in (reserve_zec, zec_usd, spend_usd_per_token))
            or reserve_zec < 0 or min(zec_usd, spend_usd_per_token) <= 0):
        raise ValueError('nonnegative reserve and positive finite price/cost required')
    return reserve_zec * zec_usd / spend_usd_per_token
