//! Consensus rejection tests: every rejected block must leave all state unchanged.
use serde_json::Value;
use zsv::*;

struct Fixture {
    state: State,
    owner: Wallet,
    other: Wallet,
    oracles: [Wallet; 3],
}

fn fixture() -> Fixture {
    let oracles = [
        Wallet::from_seed([1; 32]),
        Wallet::from_seed([2; 32]),
        Wallet::from_seed([3; 32]),
    ];
    let owner = Wallet::from_seed([10; 32]);
    let other = Wallet::from_seed([11; 32]);
    let config = Config::regtest(oracles.each_ref().map(|w| w.public_key()));
    let state = State::from_regtest_fixture(
        config,
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

fn oracle(f: &Fixture, sequence: u64, price: u64, valid_until: u64) -> OracleUpdate {
    OracleUpdate {
        reports: f.oracles[..2]
            .iter()
            .map(|w| OracleReport::sign_on(&f.state, w, sequence, price, valid_until))
            .collect(),
    }
}

fn next(state: &State, transactions: Vec<SignedTx>) -> Block {
    Block {
        height: state.snapshot().height + 1,
        oracle: None,
        transactions,
    }
}

fn rejects_atomically(state: &mut State, block: Block) {
    let before = state.clone();
    assert!(
        state.validate_block(&block).is_err(),
        "validation accepted an invalid block"
    );
    assert_eq!(*state, before, "validation changed the input state");
    assert!(
        state.apply_block(&block).is_err(),
        "application accepted an invalid block"
    );
    assert_eq!(*state, before, "rejection changed consensus state");
}

fn open_and_borrow(f: &mut Fixture, valid_until: u64) {
    let block = Block {
        height: 1,
        oracle: Some(oracle(f, 1, 100 * MICRO_USD, valid_until)),
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
    };
    f.state.apply_block(&block).unwrap();
}

#[test]
fn replayed_nonce_is_rejected_within_and_across_blocks() {
    let mut f = fixture();
    let tx = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: ZATOSHIS_PER_ZEC,
        }],
    );
    let duplicate = next(&f.state, vec![tx.clone(), tx.clone()]);
    rejects_atomically(&mut f.state, duplicate);
    f.state
        .apply_block(&next(&f.state, vec![tx.clone()]))
        .unwrap();
    let replay = next(&f.state, vec![tx]);
    rejects_atomically(&mut f.state, replay);
}

#[test]
fn mainnet_and_testnet_signed_transactions_are_rejected() {
    let mut f = fixture();
    for network in [Network::Mainnet, Network::Testnet] {
        let tx = f.owner.sign_for_on(
            &f.state,
            network,
            0,
            vec![Action::Transfer {
                asset: Asset::ZEC,
                to: f.other.public_key(),
                amount: 1,
            }],
        );
        let block = next(&f.state, vec![tx]);
        rejects_atomically(&mut f.state, block);
    }
}

fn change_first_amount(value: &mut Value) -> bool {
    match value {
        Value::Object(map) => {
            if let Some(amount) = map.get_mut("amount") {
                *amount = Value::from(amount.as_u64().unwrap() + 1);
                return true;
            }
            map.values_mut().any(change_first_amount)
        }
        Value::Array(items) => items.iter_mut().any(change_first_amount),
        _ => false,
    }
}

#[test]
fn signature_binds_the_action_amount() {
    let mut f = fixture();
    let tx = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    let mut encoded = serde_json::to_value(tx).unwrap();
    assert!(
        change_first_amount(&mut encoded),
        "transaction serialization has no amount field"
    );
    let tampered: SignedTx = serde_json::from_value(encoded).unwrap();
    let block = next(&f.state, vec![tampered]);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn another_valid_signer_cannot_borrow_or_withdraw_from_the_owner_vault() {
    let mut f = fixture();
    open_and_borrow(&mut f, 100);
    for action in [
        Action::Borrow {
            vault_id: 7,
            amount: MICRO_USD,
        },
        Action::Withdraw {
            vault_id: 7,
            amount: 1,
        },
        Action::Close { vault_id: 7 },
    ] {
        let tx = f.other.sign_on(&f.state, 0, vec![action]);
        let block = next(&f.state, vec![tx]);
        rejects_atomically(&mut f.state, block);
    }
}

#[test]
fn duplicate_oracle_signatures_do_not_make_a_quorum() {
    let mut f = fixture();
    let report = OracleReport::sign_on(&f.state, &f.oracles[0], 1, 100 * MICRO_USD, 100);
    rejects_atomically(
        &mut f.state,
        Block {
            height: 1,
            oracle: Some(OracleUpdate {
                reports: vec![report.clone(), report],
            }),
            transactions: vec![],
        },
    );
}

#[test]
fn unregistered_oracle_signer_does_not_make_a_quorum() {
    let mut f = fixture();
    let stranger = Wallet::from_seed([99; 32]);
    let reports = vec![
        OracleReport::sign_on(&f.state, &f.oracles[0], 1, 100 * MICRO_USD, 100),
        OracleReport::sign_on(&f.state, &stranger, 1, 100 * MICRO_USD, 100),
    ];
    rejects_atomically(
        &mut f.state,
        Block {
            height: 1,
            oracle: Some(OracleUpdate { reports }),
            transactions: vec![],
        },
    );
}

#[test]
fn inconsistent_reports_cannot_choose_an_old_or_preferred_price() {
    let mut f = fixture();
    let reports = vec![
        OracleReport::sign_on(&f.state, &f.oracles[0], 1, 100 * MICRO_USD, 100),
        OracleReport::sign_on(&f.state, &f.oracles[1], 1, 100 * MICRO_USD, 100),
        OracleReport::sign_on(&f.state, &f.oracles[2], 2, 80 * MICRO_USD, 100),
    ];
    rejects_atomically(
        &mut f.state,
        Block {
            height: 1,
            oracle: Some(OracleUpdate { reports }),
            transactions: vec![],
        },
    );
}

#[test]
fn oracle_sequence_must_advance_and_price_must_be_positive() {
    let mut f = fixture();
    let block = Block {
        height: 1,
        oracle: Some(oracle(&f, 1, 100 * MICRO_USD, 100)),
        transactions: vec![],
    };
    f.state.apply_block(&block).unwrap();
    for update in [oracle(&f, 1, 90 * MICRO_USD, 100), oracle(&f, 2, 0, 100)] {
        let block = Block {
            height: 2,
            oracle: Some(update),
            transactions: vec![],
        };
        rejects_atomically(&mut f.state, block);
    }
}

#[test]
fn oracle_update_rolls_back_with_the_rest_of_a_rejected_block() {
    let mut f = fixture();
    open_and_borrow(&mut f, 100);
    let block = Block {
        height: 2,
        oracle: Some(oracle(&f, 2, 110 * MICRO_USD, 100)),
        transactions: vec![f.owner.sign_on(
            &f.state,
            1,
            vec![Action::Open {
                vault_id: 8,
                collateral: u64::MAX,
            }],
        )],
    };
    rejects_atomically(&mut f.state, block);
}

#[test]
fn expired_price_blocks_risk_increase_but_allows_deposit_and_repayment() {
    let mut f = fixture();
    open_and_borrow(&mut f, 1);
    for action in [
        Action::Borrow {
            vault_id: 7,
            amount: MICRO_USD,
        },
        Action::Withdraw {
            vault_id: 7,
            amount: 1,
        },
    ] {
        let tx = f.owner.sign_on(&f.state, 1, vec![action]);
        let block = next(&f.state, vec![tx]);
        rejects_atomically(&mut f.state, block);
    }
    let tx = f.owner.sign_on(
        &f.state,
        1,
        vec![
            Action::Deposit {
                vault_id: 7,
                amount: ZATOSHIS_PER_ZEC,
            },
            Action::Repay {
                vault_id: 7,
                amount: 100 * MICRO_USD,
            },
        ],
    );
    f.state.apply_block(&next(&f.state, vec![tx])).unwrap();
    let snapshot = f.state.snapshot();
    assert_eq!(
        snapshot.vaults[&VaultId::new(f.owner.public_key(), 7)].collateral,
        11 * ZATOSHIS_PER_ZEC
    );
    assert_eq!(
        snapshot.vaults[&VaultId::new(f.owner.public_key(), 7)].debt,
        300 * MICRO_USD
    );
    assert_eq!(snapshot.stable_supply, 300 * MICRO_USD);
}

#[test]
fn unknown_assets_and_generic_mint_actions_are_not_wire_types() {
    assert!(serde_json::from_str::<Asset>(r#""ZSA""#).is_err());
    assert!(serde_json::from_str::<Asset>(r#""USDC""#).is_err());
    assert!(serde_json::from_str::<Action>(r#"{"Mint":{"asset":"ZUSD","amount":1}}"#).is_err());
}

#[test]
fn oversized_action_batch_is_rejected_without_spending_a_nonce() {
    let mut f = fixture();
    let actions = (0..1025)
        .map(|_| Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        })
        .collect();
    let tx = f.owner.sign_on(&f.state, 0, actions);
    let block = next(&f.state, vec![tx]);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn skipped_or_repeated_height_is_rejected() {
    let mut f = fixture();
    for height in [0, 2, u64::MAX] {
        rejects_atomically(
            &mut f.state,
            Block {
                height,
                oracle: None,
                transactions: vec![],
            },
        );
    }
}

#[test]
fn fixture_initializer_has_a_hard_nonregtest_guard() {
    let f = fixture();
    for network in [Network::Mainnet, Network::Testnet] {
        let mut config = Config::regtest(f.oracles.each_ref().map(|w| w.public_key()));
        config.network = network;
        assert!(
            State::from_regtest_fixture(config, [(f.owner.public_key(), ZATOSHIS_PER_ZEC)],)
                .is_err()
        );
    }
}

#[test]
fn oracle_expiry_cannot_be_past_or_unbounded() {
    let mut f = fixture();
    for valid_until in [0, 102, u64::MAX] {
        let block = Block {
            height: 1,
            oracle: Some(oracle(&f, 1, 100 * MICRO_USD, valid_until)),
            transactions: vec![],
        };
        rejects_atomically(&mut f.state, block);
    }
}

#[test]
fn per_transaction_action_limit_is_enforced_on_otherwise_valid_transfers() {
    let mut f = fixture();
    let actions = (0..65)
        .map(|_| Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        })
        .collect();
    let tx = f.owner.sign_on(&f.state, 0, actions);
    let block = next(&f.state, vec![tx]);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn transaction_count_limit_is_enforced_without_relying_on_duplicate_nonces() {
    let mut f = fixture();
    let transactions = (0..257)
        .map(|nonce| {
            f.owner.sign_on(
                &f.state,
                nonce,
                vec![Action::Transfer {
                    asset: Asset::ZEC,
                    to: f.other.public_key(),
                    amount: 1,
                }],
            )
        })
        .collect();
    let block = next(&f.state, transactions);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn oracle_signatures_bind_the_reported_price() {
    let mut f = fixture();
    let mut update = oracle(&f, 1, 100 * MICRO_USD, 100);
    for report in &mut update.reports {
        report.price += MICRO_USD;
    }
    rejects_atomically(
        &mut f.state,
        Block {
            height: 1,
            oracle: Some(update),
            transactions: vec![],
        },
    );
}

#[test]
fn aggregate_action_limit_rejects_individually_valid_transactions() {
    let mut f = fixture();
    let transactions = (0..17)
        .map(|nonce| {
            let actions = (0..64)
                .map(|_| Action::Transfer {
                    asset: Asset::ZEC,
                    to: f.other.public_key(),
                    amount: 1,
                })
                .collect();
            f.owner.sign_on(&f.state, nonce, actions)
        })
        .collect();
    let block = next(&f.state, transactions);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn signature_binds_nonce_even_when_the_tampered_nonce_is_current() {
    let mut f = fixture();
    let original = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    f.state
        .apply_block(&next(&f.state, vec![original.clone()]))
        .unwrap();
    let mut tampered = original;
    tampered.nonce = 1;
    let block = next(&f.state, vec![tampered]);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn signature_binds_signer_even_when_the_substituted_signer_has_funds() {
    let mut f = fixture();
    let mut tampered = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    tampered.signer = f.other.public_key();
    let block = next(&f.state, vec![tampered]);
    rejects_atomically(&mut f.state, block);
}

#[test]
fn signature_binds_the_transfer_recipient() {
    let mut f = fixture();
    let mut tampered = f.owner.sign_on(
        &f.state,
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.public_key(),
            amount: 1,
        }],
    );
    tampered.actions[0] = Action::Transfer {
        asset: Asset::ZEC,
        to: f.owner.public_key(),
        amount: 1,
    };
    let block = next(&f.state, vec![tampered]);
    rejects_atomically(&mut f.state, block);
}
