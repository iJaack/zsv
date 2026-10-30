use zsv::*;
const Z: u64 = ZATOSHIS_PER_ZEC;
const M: u64 = MICRO_USD;

fn setup(config: ReserveConfig, seed: u64) -> (State, Wallet, [Wallet; 3]) {
    let owner = Wallet::from_seed([41; 32]);
    let oracles = [
        Wallet::from_seed([51; 32]),
        Wallet::from_seed([52; 32]),
        Wallet::from_seed([53; 32]),
    ];
    let state = State::from_regtest_fixture_with_reserve(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), 1_000 * Z)],
        config,
        owner.public_key(),
        seed,
    )
    .unwrap();
    (state, owner, oracles)
}
fn oracle(state: &State, keys: &[Wallet; 3], sequence: u64, until: u64) -> OracleUpdate {
    OracleUpdate {
        reports: keys[..2]
            .iter()
            .map(|key| OracleReport::sign_on(state, key, sequence, 100 * M, until))
            .collect(),
    }
}
fn issue_and_sell(tokens: u64) -> Vec<Action> {
    vec![
        Action::Open {
            vault_id: 1,
            collateral: 25 * Z,
        },
        Action::Borrow {
            vault_id: 1,
            amount: 1_000 * M,
        },
        buy(tokens, 0, 1),
    ]
}
fn buy(tokens: u64, epoch: u64, sequence: u64) -> Action {
    Action::ReserveBuy {
        tokens,
        oracle_sequence: sequence,
        epoch,
        valid_until: 100,
        min_zec_out: 0,
    }
}
fn empty(state: &mut State) {
    state
        .apply_block(&Block {
            height: state.snapshot().height + 1,
            oracle: None,
            transactions: vec![],
        })
        .unwrap();
}
#[test]
fn vault_issuance_funds_inventory_and_roundtrip_repayment_burns_only_owned_tokens() {
    let (mut state, owner, keys) = setup(ReserveConfig::default(), 10 * Z);
    assert_eq!(state.snapshot().accounts[&owner.public_key()].zec, 990 * Z);
    assert_eq!(state.snapshot().stable_supply, 0);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &keys, 1, 100)),
            transactions: vec![owner.sign_on(&state, 0, issue_and_sell(100 * M))],
        })
        .unwrap();
    let s = state.snapshot();
    assert_eq!(s.reserve.as_ref().unwrap().reserve_tokens, 100 * M);
    assert_eq!(s.reserve.unwrap().reserve_zec, 10 * Z - 99_500_000);
    state
        .apply_block(&Block {
            height: 2,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                1,
                vec![
                    Action::ReserveSell {
                        tokens: 100 * M,
                        oracle_sequence: 1,
                        epoch: 0,
                        valid_until: 2,
                        max_zec_in: 100_500_000,
                    },
                    Action::Repay {
                        vault_id: 1,
                        amount: 1_000 * M,
                    },
                    Action::Close { vault_id: 1 },
                ],
            )],
        })
        .unwrap();
    let s = state.snapshot();
    assert_eq!(s.stable_supply, 0);
    assert!(s.vaults.is_empty());
    assert_eq!(s.accounts[&owner.public_key()].zec, 990 * Z - 1_000_000);
    let r = s.reserve.unwrap();
    assert_eq!(r.reserve_tokens, 0);
    assert_eq!(r.reserve_zec, 10 * Z + 1_000_000);
    assert_eq!(r.retained_funding_zec, 10 * Z);
    state.check_invariants().unwrap();
}
#[test]
fn only_retained_subsidy_consumes_funding_allowance_and_overflow_goes_to_nsm() {
    let c = ReserveConfig {
        funding_target_zec: 10 * Z,
        lifetime_retained_funding_cap: 11 * Z,
        funding_start_height: 1,
        funding_end_height: 4,
        funding_zec_per_block: Z,
        ..ReserveConfig::default()
    };
    let (mut state, owner, keys) = setup(c, 10 * Z);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &keys, 1, 100)),
            transactions: vec![owner.sign_on(&state, 0, issue_and_sell(100 * M))],
        })
        .unwrap();
    state
        .apply_block(&Block {
            height: 2,
            oracle: None,
            transactions: vec![owner.sign_on(&state, 1, vec![buy(100 * M, 0, 1)])],
        })
        .unwrap();
    empty(&mut state);
    let r = state.snapshot().reserve.unwrap();
    assert_eq!(r.funded_zec, 3 * Z);
    assert_eq!(r.retained_funding_zec, 11 * Z);
    assert_eq!(r.nsm_zec, 2 * Z);
    assert_eq!(r.reserve_zec, 901_000_000);
    empty(&mut state);
    assert_eq!(state.snapshot().reserve.unwrap().funded_zec, 3 * Z);
}
#[test]
fn fresh_inclusion_before_deadline_resumes_but_at_deadline_cannot_rescue() {
    let (mut state, _, keys) = setup(ReserveConfig::default(), 10 * Z);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &keys, 1, 100)),
            transactions: vec![],
        })
        .unwrap();
    for _ in 2..=9 {
        empty(&mut state);
    }
    assert_eq!(
        state.snapshot().reserve.as_ref().unwrap().paused_since,
        Some(5)
    );
    let mut rescued = state.clone();
    rescued
        .apply_block(&Block {
            height: 10,
            oracle: Some(oracle(&rescued, &keys, 2, 100)),
            transactions: vec![],
        })
        .unwrap();
    assert_eq!(rescued.snapshot().reserve.unwrap().paused_since, None);
    empty(&mut state);
    state
        .apply_block(&Block {
            height: 11,
            oracle: Some(oracle(&state, &keys, 2, 100)),
            transactions: vec![],
        })
        .unwrap();
    let r = state.snapshot().reserve.unwrap();
    assert_eq!(r.shutdown.unwrap().reason, ShutdownReason::OracleOutage);
    assert_eq!(r.reserve_zec, 0);
    assert_eq!(r.nsm_zec, 10 * Z);
    assert_eq!(r.oracle_inclusion_height, Some(11));
}
#[test]
fn signed_expiry_shortens_reserve_freshness_and_closure_uses_earliest_outage() {
    let (mut state, _, keys) = setup(ReserveConfig::default(), 10 * Z);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &keys, 1, 1)),
            transactions: vec![],
        })
        .unwrap();
    for _ in 2..=7 {
        empty(&mut state);
    }
    assert_eq!(state.snapshot().reserve.unwrap().paused_since, Some(2));
    empty(&mut state);
    assert_eq!(
        state.snapshot().reserve.unwrap().shutdown.unwrap().height,
        8
    );
}
#[test]
fn allocation_before_activation_and_after_sunset_is_diverted() {
    let c = ReserveConfig {
        activation_height: 3,
        sunset_height: 5,
        funding_start_height: 1,
        funding_end_height: 7,
        funding_zec_per_block: Z,
        ..ReserveConfig::default()
    };
    let (mut state, _, keys) = setup(c, 0);
    for _ in 1..=2 {
        empty(&mut state);
    }
    assert_eq!(state.snapshot().reserve.unwrap().nsm_zec, 2 * Z);
    state
        .apply_block(&Block {
            height: 3,
            oracle: Some(oracle(&state, &keys, 1, 100)),
            transactions: vec![],
        })
        .unwrap();
    empty(&mut state);
    assert_eq!(state.snapshot().reserve.unwrap().reserve_zec, 2 * Z);
    empty(&mut state);
    empty(&mut state);
    let r = state.snapshot().reserve.unwrap();
    assert_eq!(r.reserve_zec, 0);
    assert_eq!(r.retained_funding_zec, 2 * Z);
    assert_eq!(r.nsm_zec, 6 * Z);
    assert_eq!(r.shutdown.unwrap().reason, ShutdownReason::Sunset);
}
#[test]
fn legacy_mode_rejects_all_new_actions_without_changing_state() {
    let (state, owner, keys) = setup(ReserveConfig::default(), 10 * Z);
    let legacy = State::from_regtest_fixture(
        Config::regtest(keys.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), 1_000 * Z)],
    )
    .unwrap();
    assert_ne!(legacy.chain_domain(), state.chain_domain());
    for action in [
        buy(M, 0, 1),
        Action::ReserveSell {
            tokens: M,
            oracle_sequence: 1,
            epoch: 0,
            valid_until: 1,
            max_zec_in: Z,
        },
        Action::ReserveAuctionBuy {
            tokens: M,
            valid_until: 1,
            max_zec_in: Z,
        },
    ] {
        let mut candidate = legacy.clone();
        assert!(candidate
            .apply_block(&Block {
                height: 1,
                oracle: None,
                transactions: vec![owner.sign_on(&legacy, 0, vec![action])]
            })
            .is_err());
        assert_eq!(candidate, legacy);
    }
}
#[test]
fn auction_floor_sells_existing_inventory_and_leaves_active_debt_unchanged() {
    let c = ReserveConfig {
        sunset_height: 2,
        auction_duration_blocks: 2,
        ..ReserveConfig::default()
    };
    let (mut state, owner, keys) = setup(c, 10 * Z);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &keys, 1, 1)),
            transactions: vec![owner.sign_on(&state, 0, issue_and_sell(100 * M))],
        })
        .unwrap();
    empty(&mut state);
    let closed = state.snapshot();
    let r = closed.reserve.unwrap();
    assert_eq!(r.auction_price(2).unwrap(), Z);
    assert_eq!(r.auction_price(3).unwrap(), 50_000_001);
    assert_eq!(r.auction_price(4).unwrap(), 1);
    assert_eq!(r.auction_price(u64::MAX).unwrap(), 1);
    empty(&mut state);
    state
        .apply_block(&Block {
            height: 4,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                1,
                vec![Action::ReserveAuctionBuy {
                    tokens: 100 * M,
                    valid_until: 4,
                    max_zec_in: 100,
                }],
            )],
        })
        .unwrap();
    let s = state.snapshot();
    assert_eq!(s.reserve.as_ref().unwrap().reserve_tokens, 0);
    assert_eq!(s.reserve.unwrap().nsm_zec, r.nsm_zec + 100);
    assert_eq!(s.stable_supply, 1_000 * M);
    assert_eq!(
        s.vaults[&VaultId::new(owner.public_key(), 1)].debt,
        1_000 * M
    );
    assert_eq!(s.accounts[&owner.public_key()].zusd, 1_000 * M);
}
#[test]
fn reserve_inventory_limits_reject_the_entire_vault_issuance_block() {
    for c in [
        ReserveConfig {
            max_reserve_tokens: 99 * M,
            ..ReserveConfig::default()
        },
        ReserveConfig {
            min_reserve_zec: 10 * Z,
            ..ReserveConfig::default()
        },
    ] {
        let (state, owner, keys) = setup(c, 10 * Z);
        let mut candidate = state.clone();
        assert!(candidate
            .apply_block(&Block {
                height: 1,
                oracle: Some(oracle(&state, &keys, 1, 100)),
                transactions: vec![owner.sign_on(&state, 0, issue_and_sell(100 * M))]
            })
            .is_err());
        assert_eq!(candidate, state);
    }
}
