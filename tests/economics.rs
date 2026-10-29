use zsv::*;

fn setup() -> (State, Wallet, Wallet, [Wallet; 3]) {
    let owner = Wallet::from_seed([1; 32]);
    let buyer = Wallet::from_seed([2; 32]);
    let oracles = [
        Wallet::from_seed([11; 32]),
        Wallet::from_seed([12; 32]),
        Wallet::from_seed([13; 32]),
    ];
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [
            (owner.public_key(), 20 * ZATOSHIS_PER_ZEC),
            (buyer.public_key(), 20 * ZATOSHIS_PER_ZEC),
        ],
    )
    .unwrap();
    (state, owner, buyer, oracles)
}
fn price(state: &State, oracles: &[Wallet; 3], sequence: u64, usd: u64) -> OracleUpdate {
    OracleUpdate {
        reports: oracles[..2]
            .iter()
            .map(|w| OracleReport::sign_on(state, w, sequence, usd * MICRO_USD, 100))
            .collect(),
    }
}
fn opened() -> (State, Wallet, Wallet, [Wallet; 3]) {
    let (mut state, owner, buyer, oracles) = setup();
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(price(&state, &oracles, 1, 100)),
            transactions: vec![owner.sign_on(
                &state,
                0,
                vec![
                    Action::Open {
                        vault_id: 1,
                        collateral: 10 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Borrow {
                        vault_id: 1,
                        amount: 400 * MICRO_USD,
                    },
                ],
            )],
        })
        .unwrap();
    (state, owner, buyer, oracles)
}
#[test]
fn borrow_mints_only_native_stablecoin_and_tracks_debt() {
    let (state, owner, _, _) = opened();
    let s = state.snapshot();
    assert_eq!(s.stable_supply, 400 * MICRO_USD);
    assert_eq!(s.bad_debt, 0);
    assert_eq!(s.accounts[&owner.public_key()].zusd, 400 * MICRO_USD);
    assert_eq!(
        s.vaults[&VaultId::new(owner.public_key(), 1)].debt,
        s.stable_supply
    );
}
#[test]
fn opening_ratio_boundary_and_supply_cap_are_enforced_atomically() {
    let (state, owner, _, _) = opened();
    for amount in [1, SUPPLY_CAP] {
        let mut branch = state.clone();
        assert!(branch
            .apply_block(&Block {
                height: 2,
                oracle: None,
                transactions: vec![owner.sign_on(
                    &state,
                    1,
                    vec![Action::Borrow {
                        vault_id: 1,
                        amount
                    }]
                )]
            })
            .is_err());
        assert_eq!(branch, state);
    }
}
#[test]
fn repay_withdraw_close_restores_collateral_and_burns_supply() {
    let (mut state, owner, _, _) = opened();
    state
        .apply_block(&Block {
            height: 2,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                1,
                vec![
                    Action::Repay {
                        vault_id: 1,
                        amount: 400 * MICRO_USD,
                    },
                    Action::Withdraw {
                        vault_id: 1,
                        amount: ZATOSHIS_PER_ZEC,
                    },
                    Action::Close { vault_id: 1 },
                ],
            )],
        })
        .unwrap();
    let s = state.snapshot();
    assert_eq!(s.stable_supply, 0);
    assert!(s.vaults.is_empty());
    assert_eq!(s.accounts[&owner.public_key()].zec, 20 * ZATOSHIS_PER_ZEC);
}
#[test]
fn solvent_liquidation_burns_debt_seizes_rounded_bonus_and_returns_surplus() {
    let (mut state, owner, buyer, oracles) = opened();
    state
        .apply_block(&Block {
            height: 2,
            oracle: Some(price(&state, &oracles, 2, 50)),
            transactions: vec![
                owner.sign_on(
                    &state,
                    1,
                    vec![Action::Transfer {
                        asset: Asset::ZUSD,
                        to: buyer.public_key(),
                        amount: 400 * MICRO_USD,
                    }],
                ),
                buyer.sign_on(
                    &state,
                    0,
                    vec![Action::Liquidate {
                        vault_id: 1,
                        owner: owner.public_key(),
                        oracle_sequence: 2,
                        max_burn: 400 * MICRO_USD,
                        min_collateral: 0,
                    }],
                ),
            ],
        })
        .unwrap();
    let s = state.snapshot();
    assert_eq!(s.stable_supply, 0);
    assert_eq!(s.bad_debt, 0);
    assert!(s.vaults.is_empty());
    assert_eq!(
        s.accounts[&buyer.public_key()].zec,
        28 * ZATOSHIS_PER_ZEC + 40_000_000
    );
    assert_eq!(
        s.accounts[&owner.public_key()].zec,
        11 * ZATOSHIS_PER_ZEC + 60_000_000
    );
}
#[test]
fn crash_liquidation_records_bad_debt_and_freezes_new_borrowing() {
    let (mut state, owner, buyer, oracles) = opened();
    state
        .apply_block(&Block {
            height: 2,
            oracle: Some(price(&state, &oracles, 2, 20)),
            transactions: vec![
                owner.sign_on(
                    &state,
                    1,
                    vec![Action::Transfer {
                        asset: Asset::ZUSD,
                        to: buyer.public_key(),
                        amount: 400 * MICRO_USD,
                    }],
                ),
                buyer.sign_on(
                    &state,
                    0,
                    vec![Action::Liquidate {
                        vault_id: 1,
                        owner: owner.public_key(),
                        oracle_sequence: 2,
                        max_burn: 400 * MICRO_USD,
                        min_collateral: 0,
                    }],
                ),
            ],
        })
        .unwrap();
    let s = state.snapshot();
    assert_eq!(s.bad_debt, 209_523_810);
    assert_eq!(s.stable_supply, s.bad_debt);
    assert!(s.vaults.is_empty());
    assert_eq!(s.accounts[&buyer.public_key()].zusd, 209_523_810);
    let before = state.clone();
    assert!(state
        .apply_block(&Block {
            height: 3,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                2,
                vec![
                    Action::Open {
                        vault_id: 2,
                        collateral: 10 * ZATOSHIS_PER_ZEC
                    },
                    Action::Borrow {
                        vault_id: 2,
                        amount: 100 * MICRO_USD
                    }
                ]
            )]
        })
        .is_err());
    assert_eq!(state, before);
}
#[test]
fn liquidation_boundary_is_strictly_below_two_hundred_percent() {
    let (mut state, owner, buyer, oracles) = opened();
    let before = state.clone();
    assert!(state
        .apply_block(&Block {
            height: 2,
            oracle: Some(price(&state, &oracles, 2, 80)),
            transactions: vec![buyer.sign_on(
                &state,
                0,
                vec![Action::Liquidate {
                    vault_id: 1,
                    owner: owner.public_key(),
                    oracle_sequence: 2,
                    max_burn: 400 * MICRO_USD,
                    min_collateral: 0
                }]
            )]
        })
        .is_err());
    assert_eq!(state, before);
}
#[test]
fn debt_dust_is_rejected_unless_fully_repaid() {
    let (mut state, owner, _, _) = opened();
    let before = state.clone();
    assert!(state
        .apply_block(&Block {
            height: 2,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                1,
                vec![Action::Repay {
                    vault_id: 1,
                    amount: 350 * MICRO_USD
                }]
            )]
        })
        .is_err());
    assert_eq!(state, before);
}
#[test]
fn genesis_overflow_and_duplicate_allocations_are_rejected() {
    let (_, owner, _, oracles) = setup();
    let config = Config::regtest(oracles.each_ref().map(Wallet::public_key));
    assert!(State::from_regtest_fixture(
        config.clone(),
        [(owner.public_key(), 1), (owner.public_key(), 2)]
    )
    .is_err());
    assert!(State::from_regtest_fixture(
        config,
        [
            (owner.public_key(), u64::MAX),
            (Wallet::from_seed([4; 32]).public_key(), 1)
        ]
    )
    .is_err());
}

#[test]
fn liquidation_enforces_signed_sequence_max_burn_and_min_collateral() {
    let (state, owner, buyer, oracles) = opened();
    for (oracle_sequence, max_burn, min_collateral) in [
        (1, 400 * MICRO_USD, 0),
        (2, 400 * MICRO_USD - 1, 0),
        (2, 400 * MICRO_USD, 840_000_001),
    ] {
        let mut branch = state.clone();
        let block = Block {
            height: 2,
            oracle: Some(price(&state, &oracles, 2, 50)),
            transactions: vec![
                owner.sign_on(
                    &state,
                    1,
                    vec![Action::Transfer {
                        asset: Asset::ZUSD,
                        to: buyer.public_key(),
                        amount: 400 * MICRO_USD,
                    }],
                ),
                buyer.sign_on(
                    &state,
                    0,
                    vec![Action::Liquidate {
                        vault_id: 1,
                        owner: owner.public_key(),
                        oracle_sequence,
                        max_burn,
                        min_collateral,
                    }],
                ),
            ],
        };
        assert!(
            branch.apply_block(&block).is_err(),
            "liquidation exceeded signed intent bounds"
        );
        assert_eq!(branch, state);
    }
}
