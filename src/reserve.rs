//! Optional reserve policy for the signed regtest account ledger.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReserveConfig {
    pub epoch_blocks: u64,
    pub max_oracle_age: u64,
    pub spread: u64,
    pub epoch_zec_cap: u64,
    pub epoch_token_cap: u64,
    pub min_reserve_zec: u64,
    pub funding_target_zec: u64,
    pub max_reserve_tokens: u64,
    pub activation_height: u64,
    pub sunset_height: u64,
    pub lifetime_retained_funding_cap: u64,
    pub oracle_grace_blocks: u64,
    pub max_accounts: u64,
    pub auction_duration_blocks: u64,
    pub auction_start_zat_per_token: u64,
    pub auction_floor_zat_per_token: u64,
    pub funding_start_height: u64,
    pub funding_end_height: u64,
    pub funding_zec_per_block: u64,
}
impl Default for ReserveConfig {
    fn default() -> Self {
        Self {
            epoch_blocks: 10,
            max_oracle_age: 3,
            spread: 5_000,
            epoch_zec_cap: 10 * ZATOSHIS_PER_ZEC,
            epoch_token_cap: 1_000 * MICRO_USD,
            min_reserve_zec: 0,
            funding_target_zec: 100 * ZATOSHIS_PER_ZEC,
            max_reserve_tokens: 40_000 * MICRO_USD,
            activation_height: 0,
            sunset_height: 1_000,
            lifetime_retained_funding_cap: 200 * ZATOSHIS_PER_ZEC,
            oracle_grace_blocks: 6,
            max_accounts: 1_024,
            auction_duration_blocks: 2_000,
            auction_start_zat_per_token: ZATOSHIS_PER_ZEC,
            auction_floor_zat_per_token: 1,
            funding_start_height: 1,
            funding_end_height: 1_000,
            funding_zec_per_block: 0,
        }
    }
}
impl ReserveConfig {
    fn validate(&self) -> Result<(), Error> {
        if self.epoch_blocks == 0
            || self.spread == 0
            || self.spread >= MICRO_USD
            || self.min_reserve_zec > self.funding_target_zec
            || self.activation_height >= self.sunset_height
            || self.oracle_grace_blocks == 0
            || self.max_accounts == 0
            || self.auction_duration_blocks == 0
            || self.auction_floor_zat_per_token == 0
            || self.auction_floor_zat_per_token > self.auction_start_zat_per_token
            || self.funding_start_height == 0
            || self.funding_end_height < self.funding_start_height
        {
            return Err(Error::Invalid("invalid reserve configuration"));
        }
        Ok(())
    }
    fn funded_through(&self, height: u64) -> Result<u64, Error> {
        let end = (u128::from(height) + 1).min(u128::from(self.funding_end_height));
        let blocks = end.saturating_sub(u128::from(self.funding_start_height));
        u64::try_from(product(&[blocks, u128::from(self.funding_zec_per_block)])?)
            .map_err(|_| Error::ArithmeticOverflow)
    }
    fn domain_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        numbers(
            &mut bytes,
            &[
                self.epoch_blocks,
                self.max_oracle_age,
                self.spread,
                self.epoch_zec_cap,
                self.epoch_token_cap,
                self.min_reserve_zec,
                self.funding_target_zec,
                self.max_reserve_tokens,
                self.activation_height,
                self.sunset_height,
                self.lifetime_retained_funding_cap,
                self.oracle_grace_blocks,
                self.max_accounts,
                self.auction_duration_blocks,
                self.auction_start_zat_per_token,
                self.auction_floor_zat_per_token,
                self.funding_start_height,
                self.funding_end_height,
                self.funding_zec_per_block,
            ],
        );
        bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ShutdownReason {
    Sunset,
    OracleOutage,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShutdownReceipt {
    pub reason: ShutdownReason,
    pub height: u64,
    pub bad_debt_burn: u64,
    pub zec_to_nsm: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReserveState {
    pub config: ReserveConfig,
    pub reserve_zec: u64,
    pub reserve_tokens: u64,
    pub nsm_zec: u64,
    pub funded_zec: u64,
    pub retained_funding_zec: u64,
    pub epoch: u64,
    pub epoch_zec_out: u64,
    pub epoch_token_out: u64,
    pub lifetime_zec_out: u64,
    pub lifetime_token_out: u64,
    /// Inclusion height of the last accepted quorum. This is not market observation time.
    pub oracle_inclusion_height: Option<u64>,
    pub paused_since: Option<u64>,
    pub last_funding_height: Option<u64>,
    pub shutdown: Option<ShutdownReceipt>,
}
impl ReserveState {
    pub fn auction_price(&self, height: u64) -> Result<u64, Error> {
        self.config.validate()?;
        let shutdown = self
            .shutdown
            .as_ref()
            .ok_or(Error::Invalid("reserve auction unavailable"))?;
        let elapsed = height
            .checked_sub(shutdown.height)
            .ok_or(Error::Invalid("auction height precedes closure"))?;
        let c = &self.config;
        let remaining = c.auction_duration_blocks.saturating_sub(elapsed);
        add(
            c.auction_floor_zat_per_token,
            rounded_ratio(
                product(&[
                    u128::from(c.auction_start_zat_per_token - c.auction_floor_zat_per_token),
                    u128::from(remaining),
                ])?,
                u128::from(c.auction_duration_blocks),
                true,
            )?,
        )
    }
    pub(super) fn check_invariants(&self, height: u64, accounts: usize) -> Result<(), Error> {
        self.config.validate()?;
        if self.retained_funding_zec > self.config.lifetime_retained_funding_cap
            || accounts as u128 > u128::from(self.config.max_accounts)
            || self.epoch != height / self.config.epoch_blocks
            || self.epoch_zec_out > self.config.epoch_zec_cap
            || self.epoch_token_out > self.config.epoch_token_cap
            || self.reserve_tokens > self.config.max_reserve_tokens
            || self.epoch_zec_out > self.lifetime_zec_out
            || self.epoch_token_out > self.lifetime_token_out
            || self.funded_zec != self.config.funded_through(height)?
            || self.last_funding_height != if height == 0 { None } else { Some(height) }
            || self.oracle_inclusion_height.is_some_and(|h| h > height)
            || self.paused_since.is_some_and(|h| h > height)
            || self
                .shutdown
                .as_ref()
                .is_some_and(|s| s.height > height || self.reserve_zec != 0)
        {
            return Err(Error::InvariantViolation);
        }
        Ok(())
    }
    fn stale_start(&self, price: Option<&Price>) -> u128 {
        let start = match (self.oracle_inclusion_height, price) {
            (Some(included), Some(price)) => {
                (u128::from(included) + u128::from(self.config.max_oracle_age) + 1)
                    .min(u128::from(price.valid_until) + 1)
            }
            _ => u128::from(self.config.activation_height),
        };
        start.max(u128::from(self.config.activation_height))
    }
}
fn rounded_ratio(numerator: u128, denominator: u128, ceil: bool) -> Result<u64, Error> {
    if denominator == 0 {
        return Err(Error::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let result = quotient
        .checked_add(u128::from(ceil && !numerator.is_multiple_of(denominator)))
        .ok_or(Error::ArithmeticOverflow)?;
    u64::try_from(result).map_err(|_| Error::ArithmeticOverflow)
}

impl State {
    /// Add an immutable reserve policy and deterministic synthetic subsidy schedule.
    /// Only the named fixture donor supplies initial ZEC; initial token inventory is zero.
    pub fn from_regtest_fixture_with_reserve(
        config: Config,
        allocations: impl IntoIterator<Item = (Pubkey, u64)>,
        reserve_config: ReserveConfig,
        donor: Pubkey,
        initial_zec: u64,
    ) -> Result<Self, Error> {
        reserve_config.validate()?;
        if initial_zec > reserve_config.funding_target_zec
            || initial_zec > reserve_config.lifetime_retained_funding_cap
        {
            return Err(Error::Invalid("initial reserve exceeds funding limits"));
        }
        let mut state = Self::from_regtest_fixture(config, allocations)?;
        add(
            state.inner.fixture_zec_total,
            reserve_config.funded_through(u64::MAX)?,
        )?;
        let mut domain = Sha256::new();
        domain.update(b"zsv/model-chain/v2\0");
        domain.update(state.chain_domain());
        domain.update(reserve_config.domain_bytes());
        domain.update(donor.0);
        domain.update(initial_zec.to_le_bytes());
        state.inner.chain_domain = domain.finalize().into();
        state.debit(donor, Asset::ZEC, initial_zec)?;
        state.inner.reserve = Some(ReserveState {
            config: reserve_config,
            reserve_zec: initial_zec,
            reserve_tokens: 0,
            nsm_zec: 0,
            funded_zec: 0,
            retained_funding_zec: initial_zec,
            epoch: 0,
            epoch_zec_out: 0,
            epoch_token_out: 0,
            lifetime_zec_out: 0,
            lifetime_token_out: 0,
            oracle_inclusion_height: None,
            paused_since: None,
            last_funding_height: None,
            shutdown: None,
        });
        state.check_invariants()?;
        Ok(state)
    }
    pub(super) fn prepare_reserve_before_oracle(&mut self) -> Result<(), Error> {
        let Some(mut r) = self.inner.reserve.clone() else {
            return Ok(());
        };
        let height = self.inner.height;
        let epoch = height / r.config.epoch_blocks;
        if epoch != r.epoch {
            r.epoch = epoch;
            r.epoch_zec_out = 0;
            r.epoch_token_out = 0;
        }
        if r.shutdown.is_none() {
            let prior_stale = r
                .paused_since
                .map(u128::from)
                .unwrap_or_else(|| r.stale_start(self.inner.oracle.as_ref()));
            let reason = if height >= r.config.sunset_height {
                Some(ShutdownReason::Sunset)
            } else if height >= r.config.activation_height
                && u128::from(height) >= prior_stale + u128::from(r.config.oracle_grace_blocks)
            {
                Some(ShutdownReason::OracleOutage)
            } else {
                None
            };
            if let Some(reason) = reason {
                let burn = r.reserve_tokens.min(self.inner.bad_debt);
                let zec = r.reserve_zec;
                r.reserve_tokens = sub(r.reserve_tokens, burn)?;
                self.inner.bad_debt = sub(self.inner.bad_debt, burn)?;
                self.inner.stable_supply = sub(self.inner.stable_supply, burn)?;
                r.nsm_zec = add(r.nsm_zec, zec)?;
                r.reserve_zec = 0;
                r.shutdown = Some(ShutdownReceipt {
                    reason,
                    height,
                    bad_debt_burn: burn,
                    zec_to_nsm: zec,
                });
            }
        }
        self.inner.reserve = Some(r);
        Ok(())
    }
    pub(super) fn prepare_reserve_after_oracle(
        &mut self,
        accepted_oracle: bool,
    ) -> Result<(), Error> {
        let Some(mut r) = self.inner.reserve.clone() else {
            return Ok(());
        };
        let height = self.inner.height;
        if accepted_oracle {
            r.oracle_inclusion_height = Some(height);
        }
        if r.shutdown.is_none() && height >= r.config.activation_height {
            let stale = r.stale_start(self.inner.oracle.as_ref());
            r.paused_since = if u128::from(height) >= stale {
                let stale = u64::try_from(stale).map_err(|_| Error::ArithmeticOverflow)?;
                Some(r.paused_since.map_or(stale, |old| old.min(stale)))
            } else {
                None
            };
        }
        if r.last_funding_height == Some(height) {
            return Err(Error::Invalid("duplicate reserve block funding"));
        }
        let funded = r.config.funded_through(height)?;
        let allocation = sub(funded, r.funded_zec)?;
        let retained = if r.shutdown.is_none()
            && r.config.activation_height <= height
            && height < r.config.sunset_height
        {
            allocation
                .min(r.config.funding_target_zec.saturating_sub(r.reserve_zec))
                .min(sub(
                    r.config.lifetime_retained_funding_cap,
                    r.retained_funding_zec,
                )?)
        } else {
            0
        };
        r.reserve_zec = add(r.reserve_zec, retained)?;
        r.retained_funding_zec = add(r.retained_funding_zec, retained)?;
        r.nsm_zec = add(r.nsm_zec, sub(allocation, retained)?)?;
        r.funded_zec = funded;
        r.last_funding_height = Some(height);
        self.inner.reserve = Some(r);
        Ok(())
    }
    pub(super) fn apply_reserve_action(
        &mut self,
        signer: Pubkey,
        action: &Action,
    ) -> Result<(), Error> {
        let mut r = self
            .inner
            .reserve
            .clone()
            .ok_or(Error::Invalid("reserve is disabled"))?;
        let height = self.inner.height;
        match *action {
            Action::ReserveBuy {
                tokens,
                oracle_sequence,
                epoch,
                valid_until,
                min_zec_out: limit_zec,
            }
            | Action::ReserveSell {
                tokens,
                oracle_sequence,
                epoch,
                valid_until,
                max_zec_in: limit_zec,
            } => {
                positive(tokens)?;
                if r.shutdown.is_some()
                    || r.paused_since.is_some()
                    || height < r.config.activation_height
                    || height >= r.config.sunset_height
                    || valid_until < height
                    || epoch != r.epoch
                {
                    return Err(Error::Invalid("reserve quote context unavailable"));
                }
                let price = self.fresh_price()?;
                if self.inner.oracle.as_ref().map(|p| p.sequence) != Some(oracle_sequence)
                    || u128::from(height) >= r.stale_start(self.inner.oracle.as_ref())
                {
                    return Err(Error::Invalid("reserve oracle context changed or stale"));
                }
                let buying = matches!(action, Action::ReserveBuy { .. });
                let peg = if buying {
                    MICRO_USD - r.config.spread
                } else {
                    MICRO_USD + r.config.spread
                };
                let zec = rounded_ratio(
                    product(&[
                        u128::from(tokens),
                        u128::from(peg),
                        u128::from(ZATOSHIS_PER_ZEC),
                    ])?,
                    product(&[u128::from(MICRO_USD), u128::from(price)])?,
                    !buying,
                )?;
                positive(zec)?;
                if buying {
                    r.reserve_zec = sub(r.reserve_zec, zec)?;
                    r.reserve_tokens = add(r.reserve_tokens, tokens)?;
                    r.epoch_zec_out = add(r.epoch_zec_out, zec)?;
                    r.lifetime_zec_out = add(r.lifetime_zec_out, zec)?;
                    if zec < limit_zec
                        || r.reserve_zec < r.config.min_reserve_zec
                        || r.reserve_tokens > r.config.max_reserve_tokens
                        || r.epoch_zec_out > r.config.epoch_zec_cap
                    {
                        return Err(Error::Invalid("reserve purchase exceeds limits"));
                    }
                    self.debit(signer, Asset::ZUSD, tokens)?;
                    self.credit(signer, Asset::ZEC, zec)?;
                } else {
                    if zec > limit_zec {
                        return Err(Error::Invalid("reserve sale exceeds signed input limit"));
                    }
                    r.reserve_tokens = sub(r.reserve_tokens, tokens)?;
                    r.reserve_zec = add(r.reserve_zec, zec)?;
                    r.epoch_token_out = add(r.epoch_token_out, tokens)?;
                    r.lifetime_token_out = add(r.lifetime_token_out, tokens)?;
                    if r.epoch_token_out > r.config.epoch_token_cap {
                        return Err(Error::Invalid("reserve sale exceeds epoch limit"));
                    }
                    self.debit(signer, Asset::ZEC, zec)?;
                    self.credit(signer, Asset::ZUSD, tokens)?;
                }
            }
            Action::ReserveAuctionBuy {
                tokens,
                valid_until,
                max_zec_in,
            } => {
                positive(tokens)?;
                if valid_until < height {
                    return Err(Error::Invalid("reserve auction order expired"));
                }
                let unit_price = r.auction_price(height)?;
                let zec = rounded_ratio(
                    product(&[u128::from(tokens), u128::from(unit_price)])?,
                    u128::from(MICRO_USD),
                    true,
                )?;
                positive(zec)?;
                if zec > max_zec_in {
                    return Err(Error::Invalid("auction exceeds signed input limit"));
                }
                r.reserve_tokens = sub(r.reserve_tokens, tokens)?;
                r.nsm_zec = add(r.nsm_zec, zec)?;
                self.debit(signer, Asset::ZEC, zec)?;
                self.credit(signer, Asset::ZUSD, tokens)?;
            }
            _ => return Err(Error::Invalid("not a reserve action")),
        }
        self.inner.reserve = Some(r);
        Ok(())
    }
}
