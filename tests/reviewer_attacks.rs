//! Independent public-API attack probes and remediation regressions.
use serde_json::json;
use zsv::*;

struct Fixture {
    state: State,
    owner: Wallet,
    other: Wallet,
    oracles: [Wallet; 3],
}

fn fixture() -> Fixture {
    let owner = Wallet::from_seed([31; 32]);
    let other = Wallet::from_seed([32; 32]);
    let oracles = [
        Wallet::from_seed([41; 32]),
        Wallet::from_seed([42; 32]),
        Wallet::from_seed([43; 32]),
    ];
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [
            (owner.public_key(), 100 * ZATOSHIS_PER_ZEC),
            (other.public_key(), 100 * ZATOSHIS_PER_ZEC),
        ],
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

fn rejects(state: &mut State, attack: Block) {
    let original = state.clone();
    let commitment = state.commitment();
    assert!(state.validate_block(&attack).is_err());
    assert_eq!(state, &original);
    assert!(state.apply_block(&attack).is_err());
    assert_eq!(state, &original);
    assert_eq!(state.commitment(), commitment);
    state.check_invariants().unwrap();
}

fn open(f: &mut Fixture) {
    f.state
        .apply_block(&Block {
            height: 1,
            oracle: Some(update(f, 1, 100 * MICRO_USD, 100)),
            transactions: vec![f.owner.sign_on(
                &f.state,
                0,
                vec![
                    Action::Open {
                        vault_id: 7,
                        collateral: 10 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Borrow {
                        vault_id: 7,
                        amount: 400 * MICRO_USD,
                    },
                ],
            )],
        })
        .unwrap();
}

fn liquidation(owner: Pubkey, id: u64, sequence: u64) -> Action {
    Action::Liquidate {
        owner,
        vault_id: id,
        oracle_sequence: sequence,
        max_burn: 400 * MICRO_USD,
        min_collateral: 0,
    }
}

#[test]
fn arbitrary_assets_issuance_and_ledger_fields_are_not_transaction_inputs() {
    for asset in ["BTC", "USDC", "zUSD", "ZSA", "0", ""] {
        assert!(serde_json::from_value::<Asset>(json!(asset)).is_err());
    }
    for action in [
        json!({"Mint": {"amount": 1}}),
        json!({"RegisterAsset": {"asset": "DOGE"}}),
        json!({"Borrow": {"vault_id": 7, "amount": 1, "asset": "DOGE"}}),
    ] {
        assert!(serde_json::from_value::<Action>(action).is_err());
    }
    let f = fixture();
    let mut encoded = serde_json::to_value(block(&f.state, vec![])).unwrap();
    for field in ["stable_supply", "bad_debt", "accounts", "vaults"] {
        encoded[field] = json!(1);
        assert!(serde_json::from_value::<Block>(encoded.clone()).is_err());
        encoded.as_object_mut().unwrap().remove(field);
    }
    let checkpoint = f.state.commitment();
    let mut untrusted_snapshot = f.state.snapshot();
    untrusted_snapshot.stable_supply = u64::MAX;
    untrusted_snapshot
        .accounts
        .get_mut(&f.owner.public_key())
        .unwrap()
        .zusd = u64::MAX;
    assert_eq!(f.state.commitment(), checkpoint);
    f.state.check_invariants().unwrap();
}

#[test]
fn unknown_json_fields_are_rejected_at_each_signed_wire_boundary() {
    let f = fixture();
    let tx = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Open {
            vault_id: 7,
            collateral: 1,
        }],
    );
    let mut tx_json = serde_json::to_value(&tx).unwrap();
    tx_json["unsigned_note"] = json!("ignored?");
    assert!(serde_json::from_value::<SignedTx>(tx_json).is_err());
    let mut report = serde_json::to_value(&update(&f, 1, MICRO_USD, 100).reports[0]).unwrap();
    report["observation_height"] = json!(u64::MAX);
    assert!(serde_json::from_value::<SignedOracleReport>(report).is_err());
    assert!(
        serde_json::from_value::<OracleUpdate>(json!({"reports": [], "threshold": 0})).is_err()
    );
    assert!(serde_json::from_value::<Action>(json!({"Open": {
        "vault_id": 7, "collateral": 1, "owner": f.other.public_key()
    }}))
    .is_err());
    let mut config = serde_json::to_value(Config::regtest(
        f.oracles.each_ref().map(Wallet::public_key),
    ))
    .unwrap();
    config["mint_key"] = json!(f.owner.public_key());
    assert!(serde_json::from_value::<Config>(config).is_err());
}

#[test]
fn same_nonce_different_requests_cannot_both_execute_and_sequential_nonces_can() {
    let mut f = fixture();
    let a = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    let b = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Open {
            vault_id: 7,
            collateral: 1,
        }],
    );
    let attack = block(&f.state, vec![a.clone(), b]);
    rejects(&mut f.state, attack);
    let b = f.owner.sign_on(
        &f.state,
        1,
        vec![Action::Open {
            vault_id: 7,
            collateral: 1,
        }],
    );
    f.state.apply_block(&block(&f.state, vec![a, b])).unwrap();
    assert_eq!(f.state.snapshot().accounts[&f.owner.public_key()].nonce, 2);
    assert_eq!(
        f.state.snapshot().vaults[&VaultId::new(f.owner.public_key(), 7)].collateral,
        1
    );
}

#[test]
fn signer_substitution_and_action_reordering_do_not_preserve_signatures() {
    let mut f = fixture();
    let signed = f.owner.sign_on(
        &f.state,
        0,
        vec![
            Action::Transfer {
                asset: Asset::ZEC,
                to: f.other.public_key(),
                amount: 1,
            },
            Action::Open {
                vault_id: 7,
                collateral: 1,
            },
        ],
    );
    let mut substituted = signed.clone();
    substituted.signer = f.other.public_key();
    let attack = block(&f.state, vec![substituted]);
    rejects(&mut f.state, attack);
    let mut reordered = signed;
    reordered.actions.reverse();
    let attack = block(&f.state, vec![reordered]);
    rejects(&mut f.state, attack);
}

#[test]
fn self_transfer_preserves_both_assets_and_only_advances_the_nonce() {
    let mut f = fixture();
    open(&mut f);
    let before = f.state.snapshot();
    f.state
        .apply_block(&block(
            &f.state,
            vec![f.owner.sign_on(
                &f.state,
                1,
                vec![
                    Action::Transfer {
                        asset: Asset::ZEC,
                        to: f.owner.public_key(),
                        amount: 50 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Transfer {
                        asset: Asset::ZUSD,
                        to: f.owner.public_key(),
                        amount: 400 * MICRO_USD,
                    },
                ],
            )],
        ))
        .unwrap();
    let after = f.state.snapshot();
    assert_eq!(
        after.accounts[&f.owner.public_key()].zec,
        before.accounts[&f.owner.public_key()].zec
    );
    assert_eq!(
        after.accounts[&f.owner.public_key()].zusd,
        before.accounts[&f.owner.public_key()].zusd
    );
    assert_eq!(after.accounts[&f.owner.public_key()].nonce, 2);
    assert_eq!(after.stable_supply, before.stable_supply);
    assert_eq!(after.vaults, before.vaults);
    f.state.check_invariants().unwrap();
}

#[test]
fn malformed_transaction_and_oracle_signature_vectors_are_rejected_atomically() {
    let mut f = fixture();
    for size in [0, 1, 63, 65, 1_048_576] {
        let mut tx = f.owner.sign_on(
            &f.state,
            0,
            vec![Action::Open {
                vault_id: 7,
                collateral: 1,
            }],
        );
        tx.signature = vec![0; size];
        let attack = block(&f.state, vec![tx]);
        rejects(&mut f.state, attack);
        let mut oracle = update(&f, 1, MICRO_USD, 100);
        oracle.reports[1].signature = vec![0; size];
        rejects(
            &mut f.state,
            Block {
                height: 1,
                oracle: Some(oracle),
                transactions: vec![],
            },
        );
    }
}

#[test]
fn zero_missing_or_excess_oracle_report_counts_do_not_create_quorums() {
    let mut f = fixture();
    let report = OracleReport::sign_on(&f.state, &f.oracles[0], 1, MICRO_USD, 100);
    for count in [0, 1, 4, 1000] {
        rejects(
            &mut f.state,
            Block {
                height: 1,
                oracle: Some(OracleUpdate {
                    reports: vec![report.clone(); count],
                }),
                transactions: vec![],
            },
        );
    }
}

#[test]
fn report_network_sequence_expiry_and_signer_are_authenticated() {
    let mut f = fixture();
    for attack_kind in 0..5 {
        let mut oracle = update(&f, 1, 100 * MICRO_USD, 100);
        match attack_kind {
            0 => {
                for r in &mut oracle.reports {
                    r.sequence = 2;
                }
            }
            1 => {
                for r in &mut oracle.reports {
                    r.valid_until = 99;
                }
            }
            2 => {
                for r in &mut oracle.reports {
                    r.network = Network::Mainnet;
                }
            }
            3 => {
                oracle.reports[1].signer = f.oracles[2].public_key();
            }
            _ => {
                oracle.reports[1].signature[0] ^= 1;
            }
        }
        rejects(
            &mut f.state,
            Block {
                height: 1,
                oracle: Some(oracle),
                transactions: vec![],
            },
        );
    }
}

#[test]
fn oracle_tuple_disagreements_reject_and_report_order_has_canonical_state() {
    let mut f = fixture();
    for changed in [
        OracleReport::sign_on(&f.state, &f.oracles[1], 2, 100 * MICRO_USD, 100),
        OracleReport::sign_on(&f.state, &f.oracles[1], 1, 101 * MICRO_USD, 100),
        OracleReport::sign_on(&f.state, &f.oracles[1], 1, 100 * MICRO_USD, 99),
    ] {
        let mut oracle = update(&f, 1, 100 * MICRO_USD, 100);
        oracle.reports[1] = changed;
        rejects(
            &mut f.state,
            Block {
                height: 1,
                oracle: Some(oracle),
                transactions: vec![],
            },
        );
    }
    let mut reversed = f.state.clone();
    let mut reports = update(&f, 1, 100 * MICRO_USD, 100);
    f.state
        .apply_block(&Block {
            height: 1,
            oracle: Some(reports.clone()),
            transactions: vec![],
        })
        .unwrap();
    reports.reports.reverse();
    reversed
        .apply_block(&Block {
            height: 1,
            oracle: Some(reports),
            transactions: vec![],
        })
        .unwrap();
    assert_eq!(f.state, reversed);
    assert_eq!(f.state.commitment(), reversed.commitment());
}

#[test]
fn oracle_future_expiry_is_bounded_but_has_no_observation_start_height() {
    let mut f = fixture();
    let excessive = update(&f, 1, 100 * MICRO_USD, 102);
    rejects(
        &mut f.state,
        Block {
            height: 1,
            oracle: Some(excessive),
            transactions: vec![],
        },
    );
    let edge = update(&f, 1, 100 * MICRO_USD, 101);
    f.state
        .apply_block(&Block {
            height: 1,
            oracle: Some(edge),
            transactions: vec![],
        })
        .unwrap();
    assert_eq!(f.state.snapshot().oracle.unwrap().valid_until, 101);
    // Expiry is authenticated; actual observation time/future data is not represented.
}

#[test]
fn maximum_integer_action_amounts_do_not_wrap_or_mutate_state() {
    let mut f = fixture();
    open(&mut f);
    for action in [
        Action::Open {
            vault_id: 8,
            collateral: u64::MAX,
        },
        Action::Deposit {
            vault_id: 7,
            amount: u64::MAX,
        },
        Action::Borrow {
            vault_id: 7,
            amount: u64::MAX,
        },
        Action::Repay {
            vault_id: 7,
            amount: u64::MAX,
        },
        Action::Withdraw {
            vault_id: 7,
            amount: u64::MAX,
        },
        Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: u64::MAX,
        },
        Action::Transfer {
            asset: Asset::ZUSD,
            to: f.other.public_key(),
            amount: u64::MAX,
        },
    ] {
        let attack = block(&f.state, vec![f.owner.sign_on(&f.state, 1, vec![action])]);
        rejects(&mut f.state, attack);
    }
}

#[test]
fn maximum_total_fixture_value_survives_self_and_full_balance_transfers() {
    let f = fixture();
    let mut state = State::from_regtest_fixture(
        Config::regtest(f.oracles.each_ref().map(Wallet::public_key)),
        [(f.owner.public_key(), u64::MAX)],
    )
    .unwrap();
    state
        .apply_block(&block(
            &state,
            vec![f.owner.sign_on(
                &state,
                0,
                vec![
                    Action::Transfer {
                        asset: Asset::ZEC,
                        to: f.owner.public_key(),
                        amount: u64::MAX,
                    },
                    Action::Transfer {
                        asset: Asset::ZEC,
                        to: f.other.public_key(),
                        amount: u64::MAX,
                    },
                ],
            )],
        ))
        .unwrap();
    assert_eq!(
        state.snapshot().accounts[&f.other.public_key()].zec,
        u64::MAX
    );
    assert_eq!(state.snapshot().accounts[&f.owner.public_key()].zec, 0);
    state.check_invariants().unwrap();
}

#[test]
fn below_minimum_debt_and_dust_repayment_are_rejected_but_full_repayment_is_valid() {
    let mut f = fixture();
    let below = Block {
        height: 1,
        oracle: Some(update(&f, 1, 100 * MICRO_USD, 100)),
        transactions: vec![f.owner.sign_on(
            &f.state,
            0,
            vec![
                Action::Open {
                    vault_id: 7,
                    collateral: 10 * ZATOSHIS_PER_ZEC,
                },
                Action::Borrow {
                    vault_id: 7,
                    amount: MIN_DEBT - 1,
                },
            ],
        )],
    };
    rejects(&mut f.state, below);
    open(&mut f);
    let dust = block(
        &f.state,
        vec![f.owner.sign_on(
            &f.state,
            1,
            vec![Action::Repay {
                vault_id: 7,
                amount: 400 * MICRO_USD - MIN_DEBT + 1,
            }],
        )],
    );
    rejects(&mut f.state, dust);
    f.state
        .apply_block(&block(
            &f.state,
            vec![f.owner.sign_on(
                &f.state,
                1,
                vec![
                    Action::Repay {
                        vault_id: 7,
                        amount: 400 * MICRO_USD,
                    },
                    Action::Close { vault_id: 7 },
                ],
            )],
        ))
        .unwrap();
    assert_eq!(f.state.snapshot().stable_supply, 0);
}

#[test]
fn unaffordable_liquidation_rolls_back_price_nonce_and_prior_actions() {
    let mut f = fixture();
    open(&mut f);
    let attack = Block {
        height: 2,
        oracle: Some(update(&f, 2, 50 * MICRO_USD, 100)),
        transactions: vec![f.other.sign_on(
            &f.state,
            0,
            vec![
                Action::Transfer {
                    asset: Asset::ZEC,
                    to: f.owner.public_key(),
                    amount: 1,
                },
                liquidation(f.owner.public_key(), 7, 2),
            ],
        )],
    };
    rejects(&mut f.state, attack);
    assert_eq!(f.state.snapshot().oracle.unwrap().sequence, 1);
}

#[test]
fn self_liquidation_conserves_assets_but_can_leave_unbacked_owner_held_supply() {
    let mut f = fixture();
    open(&mut f);
    f.state
        .apply_block(&Block {
            height: 2,
            oracle: Some(update(&f, 2, 20 * MICRO_USD, 100)),
            transactions: vec![f.owner.sign_on(
                &f.state,
                1,
                vec![liquidation(f.owner.public_key(), 7, 2)],
            )],
        })
        .unwrap();
    let snapshot = f.state.snapshot();
    assert!(snapshot.vaults.is_empty());
    assert_eq!(
        snapshot.accounts[&f.owner.public_key()].zec,
        100 * ZATOSHIS_PER_ZEC
    );
    assert_eq!(snapshot.bad_debt, 209_523_810);
    assert_eq!(
        snapshot.accounts[&f.owner.public_key()].zusd,
        snapshot.bad_debt
    );
    assert_eq!(snapshot.stable_supply, snapshot.bad_debt);
    f.state.check_invariants().unwrap();
}

#[test]
fn after_bad_debt_existing_debt_can_be_repaid_and_supply_still_equals_balances() {
    let mut f = fixture();
    open(&mut f);
    f.state
        .apply_block(&block(
            &f.state,
            vec![f.other.sign_on(
                &f.state,
                0,
                vec![
                    Action::Open {
                        vault_id: 8,
                        collateral: 10 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Borrow {
                        vault_id: 8,
                        amount: 400 * MICRO_USD,
                    },
                ],
            )],
        ))
        .unwrap();
    f.state
        .apply_block(&Block {
            height: 3,
            oracle: Some(update(&f, 2, 20 * MICRO_USD, 100)),
            transactions: vec![f.owner.sign_on(
                &f.state,
                1,
                vec![liquidation(f.owner.public_key(), 7, 2)],
            )],
        })
        .unwrap();
    let frozen = block(
        &f.state,
        vec![f.other.sign_on(
            &f.state,
            1,
            vec![Action::Borrow {
                vault_id: 8,
                amount: 1,
            }],
        )],
    );
    rejects(&mut f.state, frozen);
    f.state
        .apply_block(&block(
            &f.state,
            vec![
                f.other.sign_on(
                    &f.state,
                    1,
                    vec![
                        Action::Repay {
                            vault_id: 8,
                            amount: 400 * MICRO_USD,
                        },
                        Action::Close { vault_id: 8 },
                    ],
                ),
                f.owner.sign_on(
                    &f.state,
                    2,
                    vec![Action::Transfer {
                        asset: Asset::ZUSD,
                        to: f.other.public_key(),
                        amount: 1,
                    }],
                ),
            ],
        ))
        .unwrap();
    let snapshot = f.state.snapshot();
    assert_eq!(
        snapshot.accounts.values().map(|a| a.zusd).sum::<u64>(),
        snapshot.stable_supply
    );
    assert_eq!(snapshot.stable_supply, snapshot.bad_debt);
    assert!(snapshot.vaults.is_empty());
    f.state.check_invariants().unwrap();
}

#[test]
fn owner_scoped_vault_names_prevent_third_party_squatting() {
    let mut f = fixture();
    let original_zec = f.state.snapshot().accounts[&f.other.public_key()].zec;
    f.state
        .apply_block(&block(
            &f.state,
            vec![f.other.sign_on(
                &f.state,
                0,
                vec![
                    Action::Open {
                        vault_id: 7,
                        collateral: 1,
                    },
                    Action::Close { vault_id: 7 },
                ],
            )],
        ))
        .unwrap();
    let snapshot = f.state.snapshot();
    assert!(snapshot.vaults.is_empty());
    assert_eq!(snapshot.accounts[&f.other.public_key()].zec, original_zec);
    assert!(snapshot
        .used_vault_ids
        .contains(&VaultId::new(f.other.public_key(), 7)));
    let victim = block(
        &f.state,
        vec![f.owner.sign_on(
            &f.state,
            0,
            vec![Action::Open {
                vault_id: 7,
                collateral: 10 * ZATOSHIS_PER_ZEC,
            }],
        )],
    );
    f.state.apply_block(&victim).unwrap();
    assert_eq!(
        f.state.snapshot().vaults[&VaultId::new(f.owner.public_key(), 7)].owner,
        f.owner.public_key()
    );
}

#[test]
fn prefunding_a_public_key_cannot_squat_its_nonce_or_spending_authority() {
    let mut f = fixture();
    let fresh = Wallet::from_seed([90; 32]);
    f.state
        .apply_block(&block(
            &f.state,
            vec![f.other.sign_on(
                &f.state,
                0,
                vec![Action::Transfer {
                    asset: Asset::ZEC,
                    to: fresh.public_key(),
                    amount: 1,
                }],
            )],
        ))
        .unwrap();
    assert_eq!(f.state.snapshot().accounts[&fresh.public_key()].nonce, 0);
    f.state
        .apply_block(&block(
            &f.state,
            vec![fresh.sign_on(
                &f.state,
                0,
                vec![Action::Transfer {
                    asset: Asset::ZEC,
                    to: f.owner.public_key(),
                    amount: 1,
                }],
            )],
        ))
        .unwrap();
    assert_eq!(f.state.snapshot().accounts[&fresh.public_key()].nonce, 1);
}

#[test]
fn chain_domain_rejects_transaction_and_oracle_replay_between_distinct_configurations() {
    let mut f = fixture();
    // A changed third reporter produces a distinct genesis while retaining the
    // same valid two-signer quorum, so reporter membership cannot mask replay.
    let other_keys = [
        f.oracles[0].public_key(),
        f.oracles[1].public_key(),
        Wallet::from_seed([53; 32]).public_key(),
    ];
    let mut another = State::from_regtest_fixture(
        Config::regtest(other_keys),
        [
            (f.owner.public_key(), 100 * ZATOSHIS_PER_ZEC),
            (f.other.public_key(), 100 * ZATOSHIS_PER_ZEC),
        ],
    )
    .unwrap();
    assert_ne!(f.state.commitment(), another.commitment());
    let signed = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    let oracle = update(&f, 1, 100 * MICRO_USD, 100);
    let tx_rejected = another
        .validate_block(&block(&another, vec![signed.clone()]))
        .is_err();
    let oracle_rejected = another
        .validate_block(&Block {
            height: 1,
            oracle: Some(oracle.clone()),
            transactions: vec![],
        })
        .is_err();
    assert!(tx_rejected && oracle_rejected,
        "cross-configuration replay: transaction_rejected={tx_rejected}, oracle_rejected={oracle_rejected}");
    rejects(&mut another, block(&f.state, vec![signed.clone()]));
    rejects(
        &mut another,
        Block {
            height: 1,
            oracle: Some(oracle),
            transactions: vec![],
        },
    );
    f.state.apply_block(&block(&f.state, vec![signed])).unwrap();
}

#[test]
fn rewriting_domain_to_destination_does_not_transplant_signatures() {
    let mut f = fixture();
    let mut destination = State::from_regtest_fixture(
        Config::regtest(f.oracles.each_ref().map(Wallet::public_key)),
        [
            (f.owner.public_key(), 99 * ZATOSHIS_PER_ZEC),
            (f.other.public_key(), 100 * ZATOSHIS_PER_ZEC),
        ],
    )
    .unwrap();
    assert_ne!(f.state.chain_domain(), destination.chain_domain());
    let mut tx = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    tx.chain_domain = destination.chain_domain();
    let attack = block(&destination, vec![tx]);
    rejects(&mut destination, attack);
    let mut reports = update(&f, 1, 100 * MICRO_USD, 100);
    for report in &mut reports.reports {
        report.chain_domain = destination.chain_domain();
    }
    rejects(
        &mut destination,
        Block {
            height: 1,
            oracle: Some(reports),
            transactions: vec![],
        },
    );
    // A destination-signed request is valid, so the rejection above is not an
    // unavailable account, invalid quorum membership or impossible action.
    let signed = f.owner.sign_on(
        &destination,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    destination
        .apply_block(&block(&destination, vec![signed]))
        .unwrap();
    let oracle = update(&f, 1, 100 * MICRO_USD, 100);
    f.state
        .apply_block(&Block {
            height: 1,
            oracle: Some(oracle),
            transactions: vec![],
        })
        .unwrap();
}

#[test]
fn liquidation_owner_is_signature_bound_when_local_vault_ids_coincide() {
    let mut f = fixture();
    open(&mut f);
    f.state
        .apply_block(&block(
            &f.state,
            vec![f.other.sign_on(
                &f.state,
                0,
                vec![
                    Action::Open {
                        vault_id: 7,
                        collateral: 10 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Borrow {
                        vault_id: 7,
                        amount: 400 * MICRO_USD,
                    },
                ],
            )],
        ))
        .unwrap();
    let snapshot = f.state.snapshot();
    assert_eq!(snapshot.vaults.len(), 2);
    assert_eq!(
        snapshot.vaults[&VaultId::new(f.owner.public_key(), 7)].owner,
        f.owner.public_key()
    );
    assert_eq!(
        snapshot.vaults[&VaultId::new(f.other.public_key(), 7)].owner,
        f.other.public_key()
    );
    let mut tx = f
        .owner
        .sign_on(&f.state, 1, vec![liquidation(f.other.public_key(), 7, 2)]);
    match &mut tx.actions[0] {
        Action::Liquidate { owner, .. } => *owner = f.owner.public_key(),
        _ => unreachable!(),
    }
    let oracle = update(&f, 2, 20 * MICRO_USD, 100);
    rejects(
        &mut f.state,
        Block {
            height: 3,
            oracle: Some(oracle),
            transactions: vec![tx],
        },
    );
}
