"""Reserve lifecycle experiment with a fixed retained-funding allowance.

The host still authenticates orders/prices, supplies the exact parent branch
and authorizes each block's subsidy amount. It must call on_block before any
reserve operation and commit that transition with the block. Synthetic aggregate
debt is not proof of collateral backing or a native vault settlement mechanism.
"""

from dataclasses import dataclass, replace
from types import MappingProxyType

from analysis import reserve
from analysis.reserve import U64_MAX, ZAT, Order, Policy, Prices, ReserveState


@dataclass(frozen=True)
class Config:
    policy: Policy = Policy(lifetime_zec_cap=U64_MAX, lifetime_token_cap=U64_MAX)
    lifetime_retained_funding_cap: int = 200 * ZAT
    oracle_grace_blocks: int = 6
    max_accounts: int = 1024
    auction_duration_blocks: int = 2000
    auction_start_zat_per_token: int = ZAT
    auction_floor_zat_per_token: int = 1


@dataclass(frozen=True)
class ShutdownReceipt:
    reason: str
    height: int
    bad_debt_burn: int
    zec_to_nsm: int


@dataclass(frozen=True)
class LifecycleState:
    ledger: ReserveState
    config: Config
    retained_funding_zec: int
    latest_prices: Prices | None = None
    paused_since: int | None = None
    last_funding_height: int | None = None
    closed: bool = False
    shutdown: ShutdownReceipt | None = None
    prepared_height: int | None = None


def _config(config):
    reserve._policy(config.policy)
    reserve._u64(config.lifetime_retained_funding_cap, 'retained funding cap')
    reserve._u64(config.oracle_grace_blocks, 'oracle grace', positive=True)
    reserve._u64(config.max_accounts, 'account limit', positive=True)
    reserve._u64(config.auction_duration_blocks, 'auction duration', positive=True)
    reserve._u64(config.auction_start_zat_per_token, 'auction start', positive=True)
    reserve._u64(config.auction_floor_zat_per_token, 'auction floor', positive=True)
    if (config.policy.lifetime_zec_cap != U64_MAX
            or config.policy.lifetime_token_cap != U64_MAX):
        raise ValueError('lifecycle policy has no lifetime gross trading stop')
    if config.auction_floor_zat_per_token > config.auction_start_zat_per_token:
        raise ValueError('auction floor exceeds start price')


def _validate(state):
    _config(state.config)
    reserve._validate(state.ledger)
    reserve._u64(state.retained_funding_zec, 'retained funding')
    if (state.retained_funding_zec > state.config.lifetime_retained_funding_cap
            or len(state.ledger.accounts) > state.config.max_accounts):
        raise ValueError('allocation or account limit exceeded')
    for value in (state.paused_since, state.last_funding_height, state.prepared_height):
        if value is not None:
            reserve._u64(value, 'lifecycle height')
            if value > state.ledger.height:
                raise ValueError('future lifecycle metadata')
    if state.closed != (state.shutdown is not None):
        raise ValueError('inconsistent shutdown state')
    if state.closed and state.ledger.reserve_zec:
        raise ValueError('closed reserve retains ZEC')


def bootstrap(accounts, donor, reserve_zec, reserve_tokens, config=Config(), height=0,
              bad_debt=0):
    """Transfer a named fixture endowment and count its ZEC against allocation."""
    _config(config)
    ledger = reserve.bootstrap(accounts, donor, reserve_zec, reserve_tokens,
                               config.policy, height, bad_debt)
    state = LifecycleState(ledger, config, reserve_zec)
    _validate(state)
    return state


def _advance(state, height):
    _validate(state)
    reserve._u64(height, 'block height')
    if height < state.ledger.height:
        raise ValueError('height cannot move backward on this branch')
    epoch = height // state.config.policy.epoch_blocks
    ledger = replace(state.ledger, height=height, epoch=epoch,
                     epoch_zec_out=state.ledger.epoch_zec_out if epoch == state.ledger.epoch else 0,
                     epoch_token_out=state.ledger.epoch_token_out if epoch == state.ledger.epoch else 0)
    return replace(state, ledger=ledger)


def _observation(state, prices, height):
    if prices is None:
        return state.latest_prices
    for name, value in vars(prices).items():
        reserve._u64(value, name)
    reserve._u64(prices.zec_usd, 'ZEC/USD', positive=True)
    if prices.zec_height > height:
        raise ValueError('observation is future dated')
    old = state.latest_prices
    if old is not None:
        if prices.zec_sequence < old.zec_sequence or prices.zec_height < old.zec_height:
            raise ValueError('observation cannot roll back')
        if prices.zec_sequence == old.zec_sequence and prices != old:
            raise ValueError('one sequence must name one observation')
    return prices


def _stale_start(state, prices):
    activation = state.config.policy.activation_height
    if prices is None:
        return activation
    return max(activation, prices.zec_height + state.config.policy.max_age + 1)


def _close(state, reason):
    ledger = state.ledger
    bad_burn = min(ledger.reserve_tokens, ledger.bad_debt)
    receipt = ShutdownReceipt(reason, ledger.height, bad_burn, ledger.reserve_zec)
    ledger = replace(ledger, reserve_zec=0, reserve_tokens=ledger.reserve_tokens - bad_burn,
                     nsm_zec=ledger.nsm_zec + ledger.reserve_zec,
                     bad_debt=ledger.bad_debt - bad_burn,
                     stable_supply=ledger.stable_supply - bad_burn)
    result = replace(state, ledger=ledger, closed=True, shutdown=receipt)
    _validate(result)
    return result


def on_block(state, height, prices=None):
    """Prepare canonical block context; elapsed outage closes before late data."""
    state = _advance(state, height)
    state = replace(state, prepared_height=height)
    observation = _observation(state, prices, height)
    if state.closed:
        return state
    policy = state.config.policy
    if height >= policy.sunset_height:
        return _close(state, 'sunset')
    if height < policy.activation_height:
        return replace(state, latest_prices=observation)
    prior_stale = (state.paused_since if state.paused_since is not None
                   else _stale_start(state, state.latest_prices))
    if height >= prior_stale + state.config.oracle_grace_blocks:
        return _close(state, 'oracle_outage')
    stale = _stale_start(state, observation)
    result = replace(state, latest_prices=observation,
                     paused_since=min(prior_stale, stale) if height >= stale else None)
    _validate(result)
    return result


def _prepared(state, height):
    _validate(state)
    reserve._u64(height, 'operation height')
    if height != state.ledger.height or height != state.prepared_height:
        raise ValueError('on_block must prepare the operation height')


def execute(state, order: Order, height):
    """Apply an authorized swap using the embedded policy and observation."""
    _prepared(state, height)
    if state.closed or state.paused_since is not None or state.latest_prices is None:
        raise ValueError('reserve trading is unavailable')
    ledger, receipt = reserve.execute(state.ledger, state.config.policy,
                                      state.latest_prices, order, height)
    result = replace(state, ledger=ledger)
    _validate(result)
    return result, receipt


def credit_funding(state, amount, height):
    """Credit the host-authorized block allocation exactly once at this height."""
    _prepared(state, height)
    reserve._u64(amount, 'block funding')
    if state.last_funding_height == height:
        raise ValueError('funding already credited at this height')
    ledger, config = state.ledger, state.config
    retained = 0
    if (not state.closed
            and config.policy.activation_height <= height < config.policy.sunset_height):
        retained = min(amount,
                       max(0, config.policy.funding_target_zec - ledger.reserve_zec),
                       config.lifetime_retained_funding_cap - state.retained_funding_zec)
    ledger = replace(ledger, reserve_zec=ledger.reserve_zec + retained,
                     funded_zec=ledger.funded_zec + amount,
                     nsm_zec=ledger.nsm_zec + amount - retained)
    result = replace(state, ledger=ledger,
                     retained_funding_zec=state.retained_funding_zec + retained,
                     last_funding_height=height)
    _validate(result)
    return result


def auction_price(state, height):
    """Return the scheduled zatoshi price per whole token, including the floor."""
    _validate(state)
    reserve._u64(height, 'quote height')
    if not state.closed or state.shutdown is None or height < state.shutdown.height:
        raise ValueError('wind-down has not started')
    config = state.config
    remaining = max(config.auction_duration_blocks - (height - state.shutdown.height), 0)
    return config.auction_floor_zat_per_token + reserve._ceil(
        (config.auction_start_zat_per_token - config.auction_floor_zat_per_token) * remaining,
        config.auction_duration_blocks)


def wind_down_buy(state, actor, tokens, nonce, max_zec_in, height):
    """Buy residual inventory at a public descending ZEC price without an oracle.

    The positive floor persists if nobody buys. Proceeds go to the NSM bucket;
    the sale transfers existing tokens and cannot reduce active debt.
    """
    _prepared(state, height)
    if not state.closed or state.shutdown is None:
        raise ValueError('wind-down has not started')
    for name, value in (('tokens', tokens), ('nonce', nonce), ('maximum ZEC', max_zec_in)):
        reserve._u64(value, name, positive=name == 'tokens')
    ledger = state.ledger
    if not isinstance(actor, str) or actor not in ledger.accounts:
        raise ValueError('unknown authorized actor')
    wallet = ledger.accounts[actor]
    if nonce != wallet.nonce or tokens > ledger.reserve_tokens:
        raise ValueError('wrong nonce or insufficient auction inventory')
    unit_price = auction_price(state, height)
    payment = reserve._ceil(tokens * unit_price, reserve.MICRO)
    reserve._u64(payment, 'auction payment', positive=True)
    if payment > max_zec_in or payment > wallet.zec:
        raise ValueError('auction payment exceeds user limit or balance')
    accounts = dict(ledger.accounts)
    accounts[actor] = replace(wallet, zec=wallet.zec - payment,
                              tokens=wallet.tokens + tokens, nonce=wallet.nonce + 1)
    ledger = replace(ledger, reserve_tokens=ledger.reserve_tokens - tokens,
                     nsm_zec=ledger.nsm_zec + payment, accounts=MappingProxyType(accounts))
    result = replace(state, ledger=ledger)
    _validate(result)
    return result, {'tokens': tokens, 'zec_amount': payment,
                    'unit_price_zat_per_token': unit_price}
