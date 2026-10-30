"""Exact funding arithmetic and an uncalibrated, floating-point market model.

The market model assumes every rewarded mint or repayment changes circulating
float by the same amount. It pays a borrower bounty; participants provide the
collateral and tokens. It does not model trades, liquidation or a redemption
promise. These equations are research tools, not consensus monetary arithmetic.
"""

from dataclasses import dataclass
from fractions import Fraction
import math


def _integer(value: int, name: str) -> None:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"{name} must be a nonnegative integer")


def _share(value: Fraction) -> None:
    if not isinstance(value, Fraction) or not 0 <= value <= 1:
        raise ValueError("shares must be exact Fractions between zero and one")


def _allocation(subsidy_zat: int, share: Fraction) -> int:
    _integer(subsidy_zat, "subsidy_zat")
    _share(share)
    return subsidy_zat * share.numerator // share.denominator


def tapered_share(month: int) -> Fraction:
    """Hypothetical 3% monthly taper from 8%; no terminal cutoff is implied."""
    _integer(month, "month")
    return Fraction(2, 25) * Fraction(97, 100) ** month


def split_cut(
    subsidy_zat: int,
    reference_share: Fraction,
    tapered_share: Fraction,
    incentive_fraction: Fraction,
) -> dict[str, int]:
    """Split a nonnegative cut, preserving the reference allocation exactly."""
    reference = _allocation(subsidy_zat, reference_share)
    grants = _allocation(subsidy_zat, tapered_share)
    _share(incentive_fraction)
    if tapered_share > reference_share:
        raise ValueError("a grant extension cannot be treated as a funding cut")
    cut = reference - grants
    incentives = cut * incentive_fraction.numerator // incentive_fraction.denominator
    return {"reference": reference, "grants": grants, "incentives": incentives, "nsm": cut - incentives}


def signed_budget_difference(
    subsidies_zat: list[int],
    reference_shares: list[Fraction],
    proposed_shares: list[Fraction],
) -> int:
    """Net grant reduction; extended grants after reference expiry count negative."""
    if len(subsidies_zat) != len(reference_shares) or len(subsidies_zat) != len(proposed_shares):
        raise ValueError("budget schedules must have equal lengths")
    return sum(
        _allocation(subsidy, reference) - _allocation(subsidy, proposed)
        for subsidy, reference, proposed in zip(subsidies_zat, reference_shares, proposed_shares)
    )


def _finite(value: float, name: str, minimum: float | None = 0, strict: bool = False) -> None:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError(f"{name} must be finite")
    if minimum is not None and (value < minimum or (strict and value == minimum)):
        raise ValueError(f"{name} is outside its allowed range")


def _control_parameters(gain: float, delay: int, epochs: int) -> None:
    _finite(gain, "gain")
    _integer(delay, "delay")
    _integer(epochs, "epochs")
    if delay not in (0, 1):
        raise ValueError("only zero or one epoch delay is modeled")


def ideal_errors(gain: float, delay: int, initial_error: float, epochs: int) -> list[float]:
    """Linear recurrence, with e[-1] = e[0] for the one-epoch-delay case."""
    _control_parameters(gain, delay, epochs)
    _finite(initial_error, "initial_error", minimum=None)
    errors = [initial_error]
    previous = initial_error
    for _ in range(epochs):
        error = errors[-1]
        errors.append(error - gain * (previous if delay else error))
        previous = error
    return errors


@dataclass(frozen=True)
class Scenario:
    name: str
    epochs: int = 30
    initial_supply: float = 80_000
    target_demand: float = 85_000
    depth: float = 100_000
    gain: float = 0.5
    delay: int = 0
    response: float = 1
    reserve_zec: float = 1
    funding_zec: float = 0
    zec_usd: float = 100
    reward_usd: float = 0.01
    collateral_zec: float = 2_500
    debt_ceiling: float = 100_000
    bad_debt: bool = False
    demand_step_per_epoch: float = 0


def simulate(config: Scenario) -> list[dict[str, float | int | bool]]:
    """Bounded mint/repay response with a $1-normalized linear demand curve.

    Each row records opening state and the action taken after its price. Funding
    arrives before that action. The final row has no action or funding credit.
    Locked collateral is separate from spendable reserve. Fixed-dollar bounties
    are converted to ZEC at the assumed executable price, equal here to the
    collateral oracle price. Bad debt is only a borrowing-freeze flag; liquidation
    and writeoffs are outside this model.
    """
    _control_parameters(config.gain, config.delay, config.epochs)
    for name in ("initial_supply", "target_demand", "reserve_zec", "funding_zec", "collateral_zec", "debt_ceiling", "response"):
        _finite(getattr(config, name), name)
    for name in ("depth", "zec_usd", "reward_usd"):
        _finite(getattr(config, name), name, strict=True)
    _finite(config.demand_step_per_epoch, "demand_step_per_epoch", minimum=None)
    if config.response > 1 or config.initial_supply > config.debt_ceiling:
        raise ValueError("response or initial debt exceeds its limit")
    if not isinstance(config.bad_debt, bool):
        raise ValueError("bad_debt must be boolean")

    supply = debt = config.initial_supply
    reserve = config.reserve_zec
    demand = config.target_demand
    previous_error = (demand - supply) / config.depth
    rows = []
    for epoch in range(config.epochs + 1):
        error = (demand - supply) / config.depth
        price = 1 + error
        if price < 0 or not math.isfinite(price):
            raise ValueError("linear demand left the nonnegative-price domain")
        headroom = max(0, min(config.collateral_zec * config.zec_usd / 2.5 - debt, config.debt_ceiling - debt))
        if config.bad_debt:
            headroom = 0
        action = payout = funding = 0
        closing_reserve = reserve
        capacity_bound = False
        if epoch < config.epochs:
            funding = config.funding_zec
            available = reserve + funding
            desired = config.gain * config.depth * (previous_error if config.delay else error) * config.response
            budget_capacity = available * config.zec_usd / config.reward_usd
            action = min(desired, headroom, budget_capacity) if desired >= 0 else -min(-desired, debt, budget_capacity)
            payout = config.reward_usd * abs(action)
            closing_reserve = max(0, available - payout / config.zec_usd)
            capacity_bound = abs(action - desired) > 1e-9
        rows.append({
            "epoch": epoch, "price": price, "error": error, "supply": supply,
            "debt": debt, "demand": demand, "reserve_zec": reserve,
            "funding_zec": funding, "reserve_after_zec": closing_reserve,
            "payout_usd": payout, "action": action,
            "collateral_headroom": headroom, "capacity_bound": capacity_bound,
        })
        if epoch < config.epochs:
            supply += action
            debt += action
            reserve = closing_reserve
            demand += config.demand_step_per_epoch
            previous_error = error
    return rows
