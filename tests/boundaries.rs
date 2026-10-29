use zsv::*;

fn base(zec: u64) -> (State, Wallet, Wallet, [Wallet; 3]) {
    let owner = Wallet::from_seed([21; 32]);
    let liquidator = Wallet::from_seed([22; 32]);
    let oracles = [
        Wallet::from_seed([31; 32]),
        Wallet::from_seed([32; 32]),
        Wallet::from_seed([33; 32]),
    ];
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), zec)],
    )
    .unwrap();
    (state, owner, liquidator, oracles)
}
fn oracle(state: &State, keys: &[Wallet; 3], sequence: u64, price: u64) -> OracleUpdate {
    OracleUpdate {
        reports: keys[..2]
            .iter()
            .map(|w| OracleReport::sign_on(state, w, sequence, price, 100))
            .collect(),
    }
}

#[test]
fn exact_supply_cap_is_allowed_but_one_micro_usd_more_is_not() {
    let (mut state, owner, _, oracles) = base(3_000 * ZATOSHIS_PER_ZEC);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &oracles, 1, 100 * MICRO_USD)),
            transactions: vec![owner.sign_on(
                &state,
                0,
                vec![
                    Action::Open {
                        vault_id: 1,
                        collateral: 3_000 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Borrow {
                        vault_id: 1,
                        amount: SUPPLY_CAP,
                    },
                ],
            )],
        })
        .unwrap();
    let before = state.clone();
    let result = state.apply_block(&Block {
        height: 2,
        oracle: None,
        transactions: vec![owner.sign_on(
            &state,
            1,
            vec![Action::Borrow {
                vault_id: 1,
                amount: 1,
            }],
        )],
    });
    assert_eq!(result, Err(Error::Invalid("stablecoin supply cap")));
    assert_eq!(state, before);
}
#[test]
fn zero_incentive_dust_closes_with_bad_debt_and_returns_collateral() {
    let (mut state, owner, liquidator, oracles) = base(100_000);
    state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle(&state, &oracles, 1, 250_000 * MICRO_USD)),
            transactions: vec![owner.sign_on(
                &state,
                0,
                vec![
                    Action::Open {
                        vault_id: 1,
                        collateral: 100_000,
                    },
                    Action::Borrow {
                        vault_id: 1,
                        amount: MIN_DEBT,
                    },
                ],
            )],
        })
        .unwrap();
    state
        .apply_block(&Block {
            height: 2,
            oracle: Some(oracle(&state, &oracles, 2, 1)),
            transactions: vec![liquidator.sign_on(
                &state,
                0,
                vec![Action::Liquidate {
                    vault_id: 1,
                    owner: owner.public_key(),
                    oracle_sequence: 2,
                    max_burn: 400 * MICRO_USD,
                    min_collateral: 0,
                }],
            )],
        })
        .unwrap();
    let snapshot = state.snapshot();
    assert_eq!(snapshot.bad_debt, MIN_DEBT);
    assert_eq!(snapshot.stable_supply, MIN_DEBT);
    assert!(snapshot.vaults.is_empty());
    assert_eq!(snapshot.accounts[&owner.public_key()].zec, 100_000);
    assert_eq!(snapshot.accounts[&liquidator.public_key()].zec, 0);
}
#[test]
fn ratio_arithmetic_overflow_rejects_instead_of_wrapping() {
    let (mut state, owner, _, oracles) = base(u64::MAX);
    let before = state.clone();
    let result = state.apply_block(&Block {
        height: 1,
        oracle: Some(oracle(&state, &oracles, 1, u64::MAX)),
        transactions: vec![owner.sign_on(
            &state,
            0,
            vec![
                Action::Open {
                    vault_id: 1,
                    collateral: u64::MAX,
                },
                Action::Borrow {
                    vault_id: 1,
                    amount: MIN_DEBT,
                },
            ],
        )],
    });
    assert_eq!(result, Err(Error::ArithmeticOverflow));
    assert_eq!(state, before);
}
#[test]
fn validation_and_application_agree_and_checkpoint_clones_support_model_rollback() {
    let (mut state, owner, other, _) = base(ZATOSHIS_PER_ZEC);
    let checkpoint = state.clone();
    let initial = state.commitment();
    let block = Block {
        height: 1,
        oracle: None,
        transactions: vec![owner.sign_on(
            &state,
            0,
            vec![Action::Transfer {
                asset: Asset::ZEC,
                to: other.public_key(),
                amount: 1,
            }],
        )],
    };
    let validated = state.validate_block(&block).unwrap();
    assert_eq!(state, checkpoint);
    state.apply_block(&block).unwrap();
    assert_eq!(state, validated);
    assert_ne!(state.commitment(), initial);
    let mut replay = checkpoint;
    replay.apply_block(&block).unwrap();
    assert_eq!(state, replay);
    assert_eq!(state.commitment(), replay.commitment());
}
#[test]
fn closed_vault_ids_cannot_be_reused() {
    let (mut state, owner, _, _) = base(ZATOSHIS_PER_ZEC);
    state
        .apply_block(&Block {
            height: 1,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                0,
                vec![
                    Action::Open {
                        vault_id: 1,
                        collateral: 1,
                    },
                    Action::Close { vault_id: 1 },
                ],
            )],
        })
        .unwrap();
    let before = state.clone();
    assert!(state
        .apply_block(&Block {
            height: 2,
            oracle: None,
            transactions: vec![owner.sign_on(
                &state,
                1,
                vec![Action::Open {
                    vault_id: 1,
                    collateral: 1
                }]
            )]
        })
        .is_err());
    assert_eq!(state, before);
}

#[test]
fn chain_domain_is_canonical_and_stable_across_same_genesis_forks() {
    let (_, owner, other, oracles) = base(100);
    let keys = oracles.each_ref().map(Wallet::public_key);
    let allocations = [(owner.public_key(), 100), (other.public_key(), 50)];
    let mut state = State::from_regtest_fixture(Config::regtest(keys), allocations).unwrap();
    let mut reversed_keys = keys;
    reversed_keys.reverse();
    let equivalent = State::from_regtest_fixture(
        Config::regtest(reversed_keys),
        allocations.into_iter().rev(),
    )
    .unwrap();
    assert_eq!(state.chain_domain(), equivalent.chain_domain());
    assert_eq!(state, equivalent);
    let domain = state.chain_domain();
    let initial_checkpoint = state.commitment();
    let block = Block {
        height: 1,
        oracle: None,
        transactions: vec![owner.sign_on(
            &state,
            0,
            vec![Action::Transfer {
                asset: Asset::ZEC,
                to: other.public_key(),
                amount: 1,
            }],
        )],
    };
    let fork = equivalent.validate_block(&block).unwrap();
    state.apply_block(&block).unwrap();
    assert_eq!(state, fork);
    assert_eq!(state.chain_domain(), domain);
    assert_ne!(state.commitment(), initial_checkpoint);
    let different_allocations = State::from_regtest_fixture(
        Config::regtest(keys),
        [(owner.public_key(), 101), (other.public_key(), 50)],
    )
    .unwrap();
    assert_ne!(domain, different_allocations.chain_domain());
    assert!(different_allocations.validate_block(&block).is_err());
}
