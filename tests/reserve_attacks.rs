//! Independent attack probes against the signed vault and reserve ledger.
use serde_json::json;
use zsv::*;

const Z: u64 = ZATOSHIS_PER_ZEC;
const T: u64 = MICRO_USD;

struct Fixture {
    state: State,
    owner: Wallet,
    other: Wallet,
    oracles: [Wallet; 3],
}

fn fixture(policy: ReserveConfig) -> Fixture {
    let owner = Wallet::from_seed([71; 32]);
    let other = Wallet::from_seed([72; 32]);
    let oracles = [
        Wallet::from_seed([73; 32]),
        Wallet::from_seed([74; 32]),
        Wallet::from_seed([75; 32]),
    ];
    let state = State::from_regtest_fixture_with_reserve(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [
            (owner.public_key(), 1_000 * Z),
            (other.public_key(), 1_000 * Z),
        ],
        policy,
        owner.public_key(),
        100 * Z,
    )
    .unwrap();
    Fixture {
        state,
        owner,
        other,
        oracles,
    }
}

fn update(f: &Fixture, sequence: u64, price: u64, expiry: u64) -> OracleUpdate {
    OracleUpdate {
        reports: f.oracles[..2]
            .iter()
            .map(|key| OracleReport::sign_on(&f.state, key, sequence, price, expiry))
            .collect(),
    }
}

fn block(state: &State, transactions: Vec<SignedTx>) -> Block {
    Block {
        height: state.snapshot().height + 1,
        oracle: None,
        transactions,
    }
}

fn reserve(state: &State) -> ReserveState {
    state.snapshot().reserve.unwrap()
}

fn reject(state: &mut State, candidate: Block) {
    let before = state.clone();
    let hash = state.commitment();
    assert!(state.validate_block(&candidate).is_err());
    assert_eq!(state, &before);
    assert!(state.apply_block(&candidate).is_err());
    assert_eq!(state, &before);
    assert_eq!(state.commitment(), hash);
    state.check_invariants().unwrap();
}

fn buy(tokens: u64, sequence: u64, epoch: u64, expiry: u64) -> Action {
    Action::ReserveBuy {
        tokens,
        oracle_sequence: sequence,
        epoch,
        valid_until: expiry,
        min_zec_out: 0,
    }
}

fn sell(tokens: u64, sequence: u64, epoch: u64, expiry: u64) -> Action {
    Action::ReserveSell {
        tokens,
        oracle_sequence: sequence,
        epoch,
        valid_until: expiry,
        max_zec_in: u64::MAX,
    }
}

fn mint_and_stock(f: &mut Fixture, inventory: u64, other_debt: bool) {
    let mut actions = vec![
        Action::Open {
            vault_id: 1,
            collateral: 25 * Z,
        },
        Action::Borrow {
            vault_id: 1,
            amount: 1_000 * T,
        },
    ];
    if inventory > 0 {
        actions.push(buy(inventory, 1, 0, 1));
    }
    let mut transactions = vec![f.owner.sign_on(&f.state, 0, actions)];
    if other_debt {
        transactions.push(f.other.sign_on(
            &f.state,
            0,
            vec![
                Action::Open {
                    vault_id: 2,
                    collateral: 100 * Z,
                },
                Action::Borrow {
                    vault_id: 2,
                    amount: 4_000 * T,
                },
            ],
        ));
    }
    f.state
        .apply_block(&Block {
            height: 1,
            oracle: Some(update(f, 1, 100 * T, 100)),
            transactions,
        })
        .unwrap();
}

fn advance(state: &mut State, height: u64) {
    while state.snapshot().height < height {
        state.apply_block(&block(state, vec![])).unwrap();
    }
}

#[test]
fn each_policy_field_and_initial_endowment_changes_the_signature_domain() {
    let base = fixture(ReserveConfig::default());
    let encoded = serde_json::to_value(ReserveConfig::default()).unwrap();
    for field in encoded.as_object().unwrap().keys() {
        let mut changed = encoded.clone();
        changed[field] = json!(changed[field].as_u64().unwrap() + 1);
        let policy: ReserveConfig = serde_json::from_value(changed).unwrap();
        let mut other = fixture(policy);
        assert_ne!(
            base.state.chain_domain(),
            other.state.chain_domain(),
            "{field}"
        );
        let tx = base.owner.sign_on(
            &base.state,
            0,
            vec![Action::Transfer {
                asset: Asset::ZEC,
                to: base.other.public_key(),
                amount: 1,
            }],
        );
        let candidate = block(&other.state, vec![tx]);
        assert_eq!(
            other.state.validate_block(&candidate),
            Err(Error::Invalid("transaction chain domain"))
        );
        reject(&mut other.state, candidate);
        let candidate = Block {
            height: 1,
            oracle: Some(update(&base, 1, 100 * T, 100)),
            transactions: vec![],
        };
        assert_eq!(
            other.state.validate_block(&candidate),
            Err(Error::Invalid("oracle chain domain"))
        );
    }
    for (donor, seed) in [
        (base.owner.public_key(), 99 * Z),
        (base.other.public_key(), 100 * Z),
    ] {
        let state = State::from_regtest_fixture_with_reserve(
            Config::regtest(base.oracles.each_ref().map(Wallet::public_key)),
            [
                (base.owner.public_key(), 1_000 * Z),
                (base.other.public_key(), 1_000 * Z),
            ],
            ReserveConfig::default(),
            donor,
            seed,
        )
        .unwrap();
        assert_ne!(base.state.chain_domain(), state.chain_domain());
    }
}

#[test]
fn every_reserve_order_field_is_signed() {
    let mut f = fixture(ReserveConfig::default());
    let actions = [
        buy(100 * T, 1, 0, 10),
        sell(100 * T, 1, 0, 10),
        Action::ReserveAuctionBuy {
            tokens: 100 * T,
            valid_until: 10,
            max_zec_in: 100 * Z,
        },
    ];
    for action in actions {
        let encoded = serde_json::to_value(&action).unwrap();
        let (variant, fields) = encoded.as_object().unwrap().iter().next().unwrap();
        for field in fields.as_object().unwrap().keys() {
            let mut changed = encoded.clone();
            let value = changed[variant][field].as_u64().unwrap();
            changed[variant][field] = json!(value ^ 1);
            let mut tx = f.owner.sign_on(&f.state, 0, vec![action.clone()]);
            tx.actions[0] = serde_json::from_value(changed).unwrap();
            let candidate = block(&f.state, vec![tx]);
            assert_eq!(
                f.state.validate_block(&candidate),
                Err(Error::Invalid("invalid signature")),
                "{variant}.{field}"
            );
            reject(&mut f.state, candidate);
        }
    }
}

#[test]
fn callers_cannot_create_funding_inventory_or_another_asset() {
    for action in [
        json!({"CreditFunding": {"amount": 1}}),
        json!({"ReserveDonate": {"tokens": 1}}),
        json!({"ReserveBuy": {"tokens": 1, "oracle_sequence": 1, "epoch": 0,
            "valid_until": 10, "min_zec_out": 0, "asset": "MEME"}}),
    ] {
        assert!(serde_json::from_value::<Action>(action).is_err());
    }
    let f = fixture(ReserveConfig::default());
    let mut encoded = serde_json::to_value(block(&f.state, vec![])).unwrap();
    encoded["funding_zec"] = json!(100 * Z);
    assert!(serde_json::from_value::<Block>(encoded).is_err());
    assert_eq!(reserve(&f.state).reserve_tokens, 0);
    assert_eq!(f.state.snapshot().stable_supply, 0);
    let commitment = f.state.commitment();
    let mut detached = f.state.snapshot();
    detached.reserve.as_mut().unwrap().reserve_tokens = u64::MAX;
    assert_eq!(f.state.commitment(), commitment);
}

#[test]
fn later_failure_rolls_back_funding_and_valid_reserve_fill() {
    let mut f = fixture(ReserveConfig {
        funding_zec_per_block: Z,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 100 * T, false);
    let tx = f.owner.sign_on(
        &f.state,
        1,
        vec![
            buy(100 * T, 1, 0, 2),
            Action::Repay {
                vault_id: 1,
                amount: 1_000 * T,
            },
        ],
    );
    let candidate = block(&f.state, vec![tx]);
    reject(&mut f.state, candidate);
    assert_eq!(reserve(&f.state).funded_zec, Z);
    assert_eq!(reserve(&f.state).last_funding_height, Some(1));
    f.state.apply_block(&block(&f.state, vec![])).unwrap();
    assert_eq!(reserve(&f.state).funded_zec, 2 * Z);
    assert_eq!(reserve(&f.state).last_funding_height, Some(2));
}

#[test]
fn competing_fills_share_the_gross_epoch_cap_and_roll_back_together() {
    let mut f = fixture(ReserveConfig::default());
    mint_and_stock(&mut f, 1_000 * T, false);
    let first = f.owner.sign_on(&f.state, 1, vec![sell(600 * T, 1, 0, 2)]);
    let second = f.other.sign_on(&f.state, 0, vec![sell(600 * T, 1, 0, 2)]);
    let candidate = block(&f.state, vec![first, second]);
    reject(&mut f.state, candidate);
    let tx = f.owner.sign_on(&f.state, 1, vec![sell(1_000 * T, 1, 0, 2)]);
    f.state.apply_block(&block(&f.state, vec![tx])).unwrap();
    assert_eq!(reserve(&f.state).epoch_token_out, 1_000 * T);
    assert_eq!(reserve(&f.state).reserve_tokens, 0);
}

#[test]
fn round_trip_sale_proceeds_do_not_refund_gross_or_funding_counters() {
    let mut f = fixture(ReserveConfig::default());
    mint_and_stock(&mut f, 0, false);
    let tx = f.owner.sign_on(
        &f.state,
        1,
        vec![
            buy(500 * T, 1, 0, 2),
            sell(500 * T, 1, 0, 2),
            buy(600 * T, 1, 0, 2),
        ],
    );
    let candidate = block(&f.state, vec![tx]);
    reject(&mut f.state, candidate);
    let tx = f.owner.sign_on(
        &f.state,
        1,
        vec![buy(500 * T, 1, 0, 2), sell(500 * T, 1, 0, 2)],
    );
    f.state.apply_block(&block(&f.state, vec![tx])).unwrap();
    let r = reserve(&f.state);
    assert_eq!(r.reserve_zec, 100 * Z + Z / 20);
    assert_eq!(r.epoch_zec_out, 497_500_000);
    assert_eq!(r.epoch_token_out, 500 * T);
    assert_eq!(r.retained_funding_zec, 100 * Z);
}

#[test]
fn expired_sequence_epoch_and_nonce_are_independent_rejection_conditions() {
    let mut f = fixture(ReserveConfig::default());
    mint_and_stock(&mut f, 100 * T, false);
    for (nonce, action) in [
        (1, buy(T, 0, 0, 2)),
        (1, buy(T, 1, 1, 2)),
        (1, buy(T, 1, 0, 1)),
        (0, buy(T, 1, 0, 2)),
    ] {
        let tx = f.owner.sign_on(&f.state, nonce, vec![action]);
        let candidate = block(&f.state, vec![tx]);
        reject(&mut f.state, candidate);
    }
    let tx = f.owner.sign_on(&f.state, 1, vec![buy(T, 1, 0, 2)]);
    let candidate = block(&f.state, vec![tx.clone(), tx]);
    reject(&mut f.state, candidate);
}

#[test]
fn terminal_burn_cannot_erase_active_principal_and_late_failure_restores_it() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 3,
        funding_zec_per_block: Z,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 1_000 * T, true);
    let liquidation = f.other.sign_on(
        &f.state,
        1,
        vec![Action::Liquidate {
            vault_id: 1,
            owner: f.owner.public_key(),
            oracle_sequence: 2,
            max_burn: 1_000 * T,
            min_collateral: 0,
        }],
    );
    f.state
        .apply_block(&Block {
            height: 2,
            oracle: Some(update(&f, 2, 10 * T, 100)),
            transactions: vec![liquidation],
        })
        .unwrap();
    let before = f.state.snapshot();
    assert_eq!(before.bad_debt, 761_904_762);
    let tx = f.owner.sign_on(
        &f.state,
        1,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: u64::MAX,
        }],
    );
    let candidate = block(&f.state, vec![tx]);
    reject(&mut f.state, candidate);
    assert!(reserve(&f.state).shutdown.is_none());
    f.state.apply_block(&block(&f.state, vec![])).unwrap();
    let after = f.state.snapshot();
    let r = after.reserve.unwrap();
    assert_eq!(after.vaults, before.vaults);
    assert_eq!(after.bad_debt, 0);
    assert_eq!(after.stable_supply, before.stable_supply - before.bad_debt);
    assert_eq!(r.reserve_tokens, 1_000 * T - before.bad_debt);
    assert_eq!(r.shutdown.unwrap().bad_debt_burn, before.bad_debt);
    assert_eq!(r.reserve_zec, 0);
    assert_eq!(r.funded_zec, 3 * Z);
    assert_eq!(r.nsm_zec, 103 * Z - 995_000_000);
    f.state.check_invariants().unwrap();
}

#[test]
fn insufficient_terminal_inventory_leaves_explicit_bad_debt() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 3,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 200 * T, true);
    let liquidation = f.other.sign_on(
        &f.state,
        1,
        vec![Action::Liquidate {
            vault_id: 1,
            owner: f.owner.public_key(),
            oracle_sequence: 2,
            max_burn: 1_000 * T,
            min_collateral: 0,
        }],
    );
    f.state
        .apply_block(&Block {
            height: 2,
            oracle: Some(update(&f, 2, 10 * T, 100)),
            transactions: vec![liquidation],
        })
        .unwrap();
    f.state.apply_block(&block(&f.state, vec![])).unwrap();
    let snapshot = f.state.snapshot();
    assert_eq!(snapshot.bad_debt, 561_904_762);
    let r = snapshot.reserve.unwrap();
    assert_eq!(r.reserve_tokens, 0);
    assert_eq!(r.shutdown.unwrap().bad_debt_burn, 200 * T);
    assert!(r.nsm_zec > 0);
    let tx = f.other.sign_on(
        &f.state,
        2,
        vec![Action::Borrow {
            vault_id: 2,
            amount: T,
        }],
    );
    let candidate = block(&f.state, vec![tx]);
    assert_eq!(
        f.state.validate_block(&candidate),
        Err(Error::Invalid("borrowing frozen by bad debt"))
    );
    reject(&mut f.state, candidate);
}

#[test]
fn new_bad_debt_after_closure_does_not_repeat_the_one_time_burn() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 2,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 1_000 * T, true);
    let liquidation = f.other.sign_on(
        &f.state,
        1,
        vec![Action::Liquidate {
            vault_id: 1,
            owner: f.owner.public_key(),
            oracle_sequence: 2,
            max_burn: 1_000 * T,
            min_collateral: 0,
        }],
    );
    f.state
        .apply_block(&Block {
            height: 2,
            oracle: Some(update(&f, 2, 10 * T, 100)),
            transactions: vec![liquidation],
        })
        .unwrap();
    let r = reserve(&f.state);
    assert_eq!(r.shutdown.unwrap().bad_debt_burn, 0);
    assert_eq!(r.reserve_tokens, 1_000 * T);
    assert_eq!(f.state.snapshot().bad_debt, 761_904_762);
    advance(&mut f.state, 3);
    assert_eq!(reserve(&f.state).reserve_tokens, 1_000 * T);
    assert_eq!(f.state.snapshot().bad_debt, 761_904_762);
}

#[test]
fn fresh_report_before_deadline_resumes_but_at_deadline_cannot_rescue() {
    let mut f = fixture(ReserveConfig::default());
    mint_and_stock(&mut f, 100 * T, false);
    advance(&mut f.state, 5);
    assert_eq!(reserve(&f.state).paused_since, Some(5));
    advance(&mut f.state, 9);
    let mut before_deadline = f.state.clone();
    let report = update(&f, 2, 100 * T, 100);
    before_deadline
        .apply_block(&Block {
            height: 10,
            oracle: Some(report.clone()),
            transactions: vec![],
        })
        .unwrap();
    assert_eq!(reserve(&before_deadline).paused_since, None);
    assert!(reserve(&before_deadline).shutdown.is_none());
    advance(&mut f.state, 10);
    f.state
        .apply_block(&Block {
            height: 11,
            oracle: Some(report),
            transactions: vec![],
        })
        .unwrap();
    let r = reserve(&f.state);
    assert_eq!(r.shutdown.unwrap().reason, ShutdownReason::OracleOutage);
    assert_eq!(r.reserve_zec, 0);
    assert_eq!(r.reserve_tokens, 100 * T);
}

#[test]
fn signed_expiry_starts_outage_even_when_inclusion_age_is_small() {
    let mut f = fixture(ReserveConfig::default());
    f.state
        .apply_block(&Block {
            height: 1,
            oracle: Some(update(&f, 1, 100 * T, 1)),
            transactions: vec![],
        })
        .unwrap();
    advance(&mut f.state, 2);
    assert_eq!(reserve(&f.state).paused_since, Some(2));
    advance(&mut f.state, 7);
    assert!(reserve(&f.state).shutdown.is_none());
    f.state
        .apply_block(&Block {
            height: 8,
            oracle: Some(update(&f, 2, 100 * T, 100)),
            transactions: vec![],
        })
        .unwrap();
    assert_eq!(reserve(&f.state).shutdown.unwrap().height, 8);
}

#[test]
fn stale_sequence_cannot_refresh_inclusion_time_or_postpone_shutdown() {
    let mut f = fixture(ReserveConfig::default());
    mint_and_stock(&mut f, 100 * T, false);
    advance(&mut f.state, 9);
    for price in [100 * T, 101 * T] {
        let candidate = Block {
            height: 10,
            oracle: Some(update(&f, 1, price, 100)),
            transactions: vec![],
        };
        reject(&mut f.state, candidate);
    }
    assert_eq!(reserve(&f.state).oracle_inclusion_height, Some(1));
    assert_eq!(reserve(&f.state).paused_since, Some(5));
    advance(&mut f.state, 11);
    assert_eq!(reserve(&f.state).shutdown.unwrap().height, 11);
}

#[test]
fn oracle_free_auction_enforces_price_expiry_and_replay_bounds() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 2,
        auction_duration_blocks: 2,
        auction_start_zat_per_token: 100,
        auction_floor_zat_per_token: 1,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 100 * T, false);
    f.state
        .apply_block(&Block {
            height: 2,
            oracle: Some(update(&f, 2, 100 * T, 2)),
            transactions: vec![],
        })
        .unwrap();
    advance(&mut f.state, 4);
    assert!(reserve(&f.state).shutdown.is_some());
    // At the floor, 100 whole tokens cost exactly 100 zatoshi.
    for (tokens, expiry, maximum) in [(100 * T, 5, 99), (100 * T, 4, 100), (100 * T + 1, 5, 101)] {
        let tx = f.other.sign_on(
            &f.state,
            0,
            vec![Action::ReserveAuctionBuy {
                tokens,
                valid_until: expiry,
                max_zec_in: maximum,
            }],
        );
        let candidate = block(&f.state, vec![tx]);
        reject(&mut f.state, candidate);
    }
    let before = f.state.snapshot();
    let tx = f.other.sign_on(
        &f.state,
        0,
        vec![Action::ReserveAuctionBuy {
            tokens: 100 * T,
            valid_until: 10,
            max_zec_in: 100,
        }],
    );
    f.state
        .apply_block(&block(&f.state, vec![tx.clone()]))
        .unwrap();
    let after = f.state.snapshot();
    assert_eq!(after.stable_supply, before.stable_supply);
    assert_eq!(after.vaults, before.vaults);
    assert_eq!(
        after.accounts[&f.other.public_key()].zec,
        before.accounts[&f.other.public_key()].zec - 100
    );
    assert_eq!(after.accounts[&f.other.public_key()].zusd, 100 * T);
    assert_eq!(reserve(&f.state).reserve_tokens, 0);
    assert_eq!(
        reserve(&f.state).nsm_zec,
        before.reserve.unwrap().nsm_zec + 100
    );
    let candidate = block(&f.state, vec![tx]);
    reject(&mut f.state, candidate);
    let tx = f.other.sign_on(
        &f.state,
        1,
        vec![Action::ReserveAuctionBuy {
            tokens: 1,
            valid_until: 10,
            max_zec_in: 1,
        }],
    );
    let candidate = block(&f.state, vec![tx]);
    reject(&mut f.state, candidate);
}

#[test]
fn fractional_auction_fill_rounds_up_and_never_transfers_tokens_for_free() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 2,
        auction_start_zat_per_token: 1,
        auction_floor_zat_per_token: 1,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 100 * T, false);
    advance(&mut f.state, 2);
    let tx = f.other.sign_on(
        &f.state,
        0,
        vec![Action::ReserveAuctionBuy {
            tokens: 1,
            valid_until: 3,
            max_zec_in: 0,
        }],
    );
    let candidate = block(&f.state, vec![tx]);
    reject(&mut f.state, candidate);
    let before = reserve(&f.state);
    let tx = f.other.sign_on(
        &f.state,
        0,
        vec![Action::ReserveAuctionBuy {
            tokens: 1,
            valid_until: 3,
            max_zec_in: 1,
        }],
    );
    f.state.apply_block(&block(&f.state, vec![tx])).unwrap();
    assert_eq!(reserve(&f.state).nsm_zec, before.nsm_zec + 1);
    assert_eq!(reserve(&f.state).reserve_tokens, before.reserve_tokens - 1);
}

#[test]
fn known_funding_overflow_is_rejected_before_a_chain_can_start() {
    let f = fixture(ReserveConfig::default());
    let result = State::from_regtest_fixture_with_reserve(
        Config::regtest(f.oracles.each_ref().map(Wallet::public_key)),
        [(f.owner.public_key(), 100 * Z)],
        ReserveConfig {
            funding_end_height: 3,
            funding_zec_per_block: u64::MAX / 2,
            ..ReserveConfig::default()
        },
        f.owner.public_key(),
        100 * Z,
    );
    assert!(result.is_err());
}

#[test]
fn terminal_funding_and_sale_proceeds_never_reopen_normal_quotes() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 2,
        funding_zec_per_block: Z,
        auction_start_zat_per_token: 1,
        auction_floor_zat_per_token: 1,
        ..ReserveConfig::default()
    });
    mint_and_stock(&mut f, 100 * T, false);
    advance(&mut f.state, 2);
    let before = reserve(&f.state);
    let tx = f.other.sign_on(
        &f.state,
        0,
        vec![Action::ReserveAuctionBuy {
            tokens: 100 * T,
            valid_until: 3,
            max_zec_in: 100,
        }],
    );
    f.state
        .apply_block(&Block {
            height: 3,
            oracle: Some(update(&f, 2, 100 * T, 100)),
            transactions: vec![tx],
        })
        .unwrap();
    let after = reserve(&f.state);
    assert_eq!(after.reserve_zec, 0);
    assert_eq!(after.nsm_zec, before.nsm_zec + Z + 100);
    assert_eq!(after.retained_funding_zec, before.retained_funding_zec);
    for action in [buy(T, 2, 0, 4), sell(T, 2, 0, 4)] {
        let tx = f.owner.sign_on(&f.state, 1, vec![action]);
        let candidate = block(&f.state, vec![tx]);
        reject(&mut f.state, candidate);
    }
}

#[test]
fn detached_auction_configuration_returns_an_error_instead_of_panicking() {
    let mut f = fixture(ReserveConfig {
        sunset_height: 1,
        ..ReserveConfig::default()
    });
    advance(&mut f.state, 1);
    let mut detached = reserve(&f.state);
    detached.config.auction_start_zat_per_token = 1;
    detached.config.auction_floor_zat_per_token = 2;
    assert!(detached.auction_price(2).is_err());
    detached.config.auction_floor_zat_per_token = 1;
    detached.config.auction_duration_blocks = 0;
    assert!(detached.auction_price(2).is_err());
    assert_eq!(reserve(&f.state).auction_price(1).unwrap(), Z);
}

#[test]
fn dust_and_extreme_orders_neither_mint_value_nor_change_state_on_rejection() {
    let mut f = fixture(ReserveConfig::default());
    mint_and_stock(&mut f, 0, false);
    for action in [
        buy(1, 1, 0, 2),
        buy(u64::MAX, 1, 0, 2),
        sell(u64::MAX, 1, 0, 2),
    ] {
        let tx = f.owner.sign_on(&f.state, 1, vec![action]);
        let candidate = block(&f.state, vec![tx]);
        reject(&mut f.state, candidate);
    }
    let before = f.state.snapshot();
    let tx = f
        .owner
        .sign_on(&f.state, 1, vec![buy(2, 1, 0, 2), sell(1, 1, 0, 2)]);
    f.state.apply_block(&block(&f.state, vec![tx])).unwrap();
    let after = f.state.snapshot();
    assert_eq!(
        after.accounts[&f.owner.public_key()].zec,
        before.accounts[&f.owner.public_key()].zec - 1
    );
    assert_eq!(
        after.accounts[&f.owner.public_key()].zusd,
        before.accounts[&f.owner.public_key()].zusd - 1
    );
    let r = reserve(&f.state);
    assert_eq!(r.reserve_zec, 100 * Z + 1);
    assert_eq!(r.reserve_tokens, 1);
    assert_eq!(r.epoch_zec_out, 1);
    assert_eq!(r.epoch_token_out, 1);
    assert_eq!(after.stable_supply, before.stable_supply);
}
