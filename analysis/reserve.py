"""Exact-accounting reserve experiment, with host-verified orders and prices.

This module does not verify signatures or authenticate an oracle. The host
supplies authorized orders, the canonical unchanged policy and exact parent
branch, and one canonical ZEC/USD value for each observation sequence. Funding
amount and cadence must come from a once-per-block subsidy schedule in the host;
credit_funding itself neither derives emissions nor prevents repeated credits.
Wide Python intermediates are exact; stored monetary values are bounded u64.
Both standing quotes transfer existing assets. Neither changes stable debt.
"""

from dataclasses import dataclass, replace
from types import MappingProxyType
from typing import Mapping

ZAT = 10**8
MICRO = 10**6
U64_MAX = 2**64 - 1


def _u64(value, name='value', positive=False):
    if (isinstance(value, bool) or not isinstance(value, int)
            or value < (1 if positive else 0) or value > U64_MAX):
        raise ValueError(f'{name} is outside its u64 range')
    return value


def _ceil(numerator, denominator):
    return (numerator + denominator - 1) // denominator


@dataclass(frozen=True)
class Wallet:
    zec: int
    tokens: int
    nonce: int = 0


@dataclass(frozen=True)
class Prices:
    zec_usd: int
    zec_sequence: int
    zec_height: int


@dataclass(frozen=True)
class Policy:
    epoch_blocks: int = 10
    max_age: int = 3
    spread: int = 5_000
    epoch_zec_cap: int = 10 * ZAT
    lifetime_zec_cap: int = 100 * ZAT
    epoch_token_cap: int = 1_000 * MICRO
    lifetime_token_cap: int = 20_000 * MICRO
    min_reserve_zec: int = 0
    funding_target_zec: int = 100 * ZAT
    max_reserve_tokens: int = 40_000 * MICRO
    activation_height: int = 0
    sunset_height: int = 1000


@dataclass(frozen=True)
class Order:
    actor: str
    side: str
    tokens: int
    nonce: int
    zec_sequence: int
    epoch: int
    valid_until: int
    limit_zec: int


@dataclass(frozen=True)
class ReserveState:
    reserve_zec: int
    reserve_tokens: int
    accounts: Mapping[str, Wallet]
    stable_supply: int
    active_debt: int
    bad_debt: int
    genesis_zec: int
    funded_zec: int = 0
    nsm_zec: int = 0
    height: int = 0
    epoch: int = 0
    epoch_zec_out: int = 0
    epoch_token_out: int = 0
    lifetime_zec_out: int = 0
    lifetime_token_out: int = 0
    last_zec_sequence: int = 0


def _policy(policy):
    for name, value in vars(policy).items():
        _u64(value, name)
    if (policy.epoch_blocks == 0 or not 0 < policy.spread < MICRO
            or policy.min_reserve_zec > policy.funding_target_zec
            or policy.activation_height >= policy.sunset_height):
        raise ValueError('invalid reserve policy')


def _validate(state):
    for name, value in vars(state).items():
        if name != 'accounts':
            _u64(value, name)
    for name, wallet in state.accounts.items():
        if not isinstance(name, str) or not name:
            raise ValueError('account names must be nonempty strings')
        for field, value in vars(wallet).items():
            _u64(value, field)
    tokens = state.reserve_tokens + sum(w.tokens for w in state.accounts.values())
    zec = state.reserve_zec + state.nsm_zec + sum(w.zec for w in state.accounts.values())
    if (tokens != state.stable_supply or tokens != state.active_debt + state.bad_debt
            or zec != state.genesis_zec + state.funded_zec):
        raise ValueError('asset or liability conservation failed')
    _u64(zec, 'total ZEC')


def bootstrap(accounts, donor, reserve_zec, reserve_tokens, policy=Policy(), height=0, bad_debt=0):
    """Create simulated debt-backed balances and transfer inventory from donor."""
    _policy(policy)
    for name, value in [('reserve_zec', reserve_zec), ('reserve_tokens', reserve_tokens),
                        ('height', height), ('bad_debt', bad_debt)]:
        _u64(value, name)
    accounts = dict(accounts)
    if donor not in accounts:
        raise ValueError('missing inventory donor')
    for wallet in accounts.values():
        for name, value in vars(wallet).items():
            _u64(value, name)
    source = accounts[donor]
    if source.zec < reserve_zec or source.tokens < reserve_tokens:
        raise ValueError('donor cannot fund reserve inventory')
    if reserve_zec > policy.funding_target_zec or reserve_tokens > policy.max_reserve_tokens:
        raise ValueError('bootstrap exceeds reserve exposure limits')
    supply = _u64(sum(w.tokens for w in accounts.values()), 'stable supply')
    if bad_debt > supply:
        raise ValueError('bad debt exceeds total liabilities')
    genesis_zec = _u64(sum(w.zec for w in accounts.values()), 'genesis ZEC')
    accounts[donor] = replace(source, zec=source.zec - reserve_zec, tokens=source.tokens - reserve_tokens)
    state = ReserveState(reserve_zec, reserve_tokens, MappingProxyType(accounts),
                         supply, supply - bad_debt, bad_debt, genesis_zec,
                         height=height, epoch=height // policy.epoch_blocks)
    _validate(state)
    return state


def _at_height(state, policy, height):
    _policy(policy)
    _validate(state)
    _u64(height, 'height')
    if height < state.height:
        raise ValueError('height cannot roll backward inside a branch transition')
    if not policy.activation_height <= height < policy.sunset_height:
        raise ValueError('reserve is outside its active window')
    epoch = height // policy.epoch_blocks
    if epoch < state.epoch:
        raise ValueError('epoch cannot roll backward')
    return replace(state, height=height, epoch=epoch,
                   epoch_zec_out=state.epoch_zec_out if epoch == state.epoch else 0,
                   epoch_token_out=state.epoch_token_out if epoch == state.epoch else 0)


def credit_funding(state, policy, amount, height):
    """Top up toward the funding target; excess subsidy goes to the NSM bucket."""
    _u64(amount, 'funding')
    state = _at_height(state, policy, height)
    available = max(0, policy.funding_target_zec - state.reserve_zec)
    retained = min(amount, available)
    result = replace(state, reserve_zec=state.reserve_zec + retained,
                     nsm_zec=state.nsm_zec + amount - retained,
                     funded_zec=state.funded_zec + amount)
    _validate(result)
    return result


def execute(state, policy, prices, order, height):
    """Atomically fill one authorized research order at the standing bid or ask."""
    state = _at_height(state, policy, height)
    for name, value in vars(prices).items():
        _u64(value, name)
    _u64(prices.zec_usd, 'ZEC/USD', positive=True)
    if (prices.zec_height > height or height - prices.zec_height > policy.max_age
            or prices.zec_sequence < state.last_zec_sequence):
        raise ValueError('stale, future-dated or rolled-back ZEC observation')
    for name in ('tokens', 'nonce', 'zec_sequence', 'epoch', 'valid_until', 'limit_zec'):
        _u64(getattr(order, name), name, positive=name == 'tokens')
    if (not isinstance(order.actor, str) or order.actor not in state.accounts
            or order.side not in ('buy', 'sell')
            or order.epoch != state.epoch or order.valid_until < height
            or order.zec_sequence != prices.zec_sequence):
        raise ValueError('invalid actor, direction or order context')
    wallet = state.accounts[order.actor]
    if order.nonce != wallet.nonce:
        raise ValueError('wrong order nonce')
    q = order.tokens
    if order.side == 'buy':
        zec = q * (MICRO - policy.spread) * ZAT // (MICRO * prices.zec_usd)
        if (zec == 0 or zec < order.limit_zec or wallet.tokens < q
                or state.reserve_zec - zec < policy.min_reserve_zec
                or state.reserve_tokens + q > policy.max_reserve_tokens):
            raise ValueError('buy amount, assets or inventory limit')
        updated = replace(wallet, zec=wallet.zec + zec, tokens=wallet.tokens - q, nonce=wallet.nonce + 1)
        result = replace(state, reserve_zec=state.reserve_zec - zec,
                         reserve_tokens=state.reserve_tokens + q,
                         epoch_zec_out=state.epoch_zec_out + zec,
                         lifetime_zec_out=state.lifetime_zec_out + zec)
    else:
        zec = _ceil(q * (MICRO + policy.spread) * ZAT, MICRO * prices.zec_usd)
        if zec == 0 or zec > order.limit_zec or wallet.zec < zec or state.reserve_tokens < q:
            raise ValueError('sell amount or assets')
        updated = replace(wallet, zec=wallet.zec - zec, tokens=wallet.tokens + q, nonce=wallet.nonce + 1)
        result = replace(state, reserve_zec=state.reserve_zec + zec,
                         reserve_tokens=state.reserve_tokens - q,
                         epoch_token_out=state.epoch_token_out + q,
                         lifetime_token_out=state.lifetime_token_out + q)
    _u64(zec, 'ZEC fill', positive=True)
    if (result.epoch_zec_out > policy.epoch_zec_cap
            or result.lifetime_zec_out > policy.lifetime_zec_cap
            or result.epoch_token_out > policy.epoch_token_cap
            or result.lifetime_token_out > policy.lifetime_token_cap):
        raise ValueError('gross reserve budget exceeded')
    accounts = dict(state.accounts)
    accounts[order.actor] = updated
    result = replace(result, accounts=MappingProxyType(accounts),
                     last_zec_sequence=prices.zec_sequence)
    _validate(result)
    return result, {'side': order.side, 'tokens': q, 'zec_amount': zec, 'nsm_overflow': 0}
