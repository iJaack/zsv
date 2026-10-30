use zebra_chain::{block, parameters::Network};
use zsv::{
    Action, Block, Config, OracleReport, OracleUpdate, ReserveConfig, State, Wallet,
    MICRO_USD as T, ZATOSHIS_PER_ZEC as Z,
};
use zsv_zebra_dev::{fixture_envelope, Harness};

fn fixture() -> (State, Wallet, [Wallet; 3]) {
    let owner = Wallet::from_seed([81; 32]);
    let reporters = [
        Wallet::from_seed([82; 32]),
        Wallet::from_seed([83; 32]),
        Wallet::from_seed([84; 32]),
    ];
    let state = State::from_regtest_fixture_with_reserve(
        Config::regtest(reporters.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), 1_000 * Z)],
        ReserveConfig {
            sunset_height: 3,
            funding_zec_per_block: Z,
            funding_start_height: 1,
            funding_end_height: 10,
            auction_duration_blocks: 2,
            ..ReserveConfig::default()
        },
        owner.public_key(),
        100 * Z,
    )
    .unwrap();
    (state, owner, reporters)
}
fn empty(height: u64) -> Block {
    Block {
        height,
        oracle: None,
        transactions: vec![],
    }
}
fn append(harness: &mut Harness, block: &Block, nonce: u64) -> block::Hash {
    let parent_hash = harness.tip();
    let state = harness.state_at(parent_hash).unwrap();
    harness
        .accept(&fixture_envelope(&state, parent_hash, block, nonce).unwrap())
        .unwrap()
}
fn stock(state: &State, owner: &Wallet, reporters: &[Wallet; 3]) -> Block {
    Block {
        height: 1,
        oracle: Some(OracleUpdate {
            reports: reporters[..2]
                .iter()
                .map(|w| OracleReport::sign_on(state, w, 1, 100 * T, 100))
                .collect(),
        }),
        transactions: vec![owner.sign_on(
            state,
            0,
            vec![
                Action::Open {
                    vault_id: 1,
                    collateral: 25 * Z,
                },
                Action::Borrow {
                    vault_id: 1,
                    amount: 1_000 * T,
                },
                Action::ReserveBuy {
                    tokens: 400 * T,
                    oracle_sequence: 1,
                    epoch: 0,
                    valid_until: 1,
                    min_zec_out: 398_000_000,
                },
            ],
        )],
    }
}

#[test]
fn native_coinbase_commitments_execute_the_signed_reserve_model() {
    let (state, owner, reporters) = fixture();
    let mut harness =
        Harness::new(&Network::new_regtest(Default::default()), state.clone()).unwrap();
    let first = stock(&state, &owner, &reporters);
    let hash = append(&mut harness, &first, 1);
    let final_state = harness.state_at(hash).unwrap();
    assert_eq!(final_state, state.validate_block(&first).unwrap());
    assert_eq!(
        final_state.snapshot().reserve.unwrap().reserve_tokens,
        400 * T
    );
    final_state.check_invariants().unwrap();
}

#[test]
fn sibling_native_blocks_retain_distinct_reserve_states_and_select_exact_parent() {
    let (state, owner, reporters) = fixture();
    let network = Network::new_regtest(Default::default());
    let mut harness = Harness::new(&network, state.clone()).unwrap();
    append(&mut harness, &stock(&state, &owner, &reporters), 1);
    let parent_hash = append(&mut harness, &empty(2), 2);
    let parent = harness.state_at(parent_hash).unwrap();
    let before = parent.snapshot();
    let no_auction = append(&mut harness, &empty(3), 3);
    harness.select_tip(parent_hash).unwrap();
    let auction = Block {
        height: 3,
        oracle: None,
        transactions: vec![owner.sign_on(
            &parent,
            1,
            vec![Action::ReserveAuctionBuy {
                tokens: T,
                valid_until: 3,
                max_zec_in: Z,
            }],
        )],
    };
    let with_auction = append(&mut harness, &auction, 4);
    assert_ne!(no_auction, with_auction);
    let a = harness.state_at(no_auction).unwrap().snapshot();
    let b = harness.state_at(with_auction).unwrap().snapshot();
    assert_eq!(harness.state_at(parent_hash).unwrap().snapshot(), before);
    assert_eq!(a.vaults, b.vaults);
    let a = a.reserve.unwrap();
    let b = b.reserve.unwrap();
    assert_eq!(a.shutdown, b.shutdown);
    assert_eq!(a.funded_zec, b.funded_zec);
    assert_eq!(a.reserve_tokens, b.reserve_tokens + T);
    assert_eq!(a.nsm_zec + Z, b.nsm_zec);
    harness.select_tip(no_auction).unwrap();
    let tip = append(&mut harness, &empty(4), 5);
    assert_eq!(
        harness
            .state_at(tip)
            .unwrap()
            .snapshot()
            .reserve
            .unwrap()
            .reserve_tokens,
        400 * T
    );
    let replay = Harness::replay(
        &network,
        state,
        &harness.journal().unwrap(),
        harness.journal_anchor().unwrap(),
    )
    .unwrap();
    assert_eq!(replay, harness);
}

#[test]
fn invalid_signed_transaction_cannot_commit_funding_or_shutdown() {
    let (state, owner, reporters) = fixture();
    let mut harness =
        Harness::new(&Network::new_regtest(Default::default()), state.clone()).unwrap();
    append(&mut harness, &stock(&state, &owner, &reporters), 1);
    let parent_hash = append(&mut harness, &empty(2), 2);
    let parent = harness.state_at(parent_hash).unwrap();
    let block = Block {
        height: 3,
        oracle: None,
        transactions: vec![owner.sign_on(
            &parent,
            1,
            vec![Action::Repay {
                vault_id: 1,
                amount: 1_000 * T,
            }],
        )],
    };
    let envelope = fixture_envelope(&parent, parent_hash, &block, 3).unwrap();
    let before = harness.clone();
    assert!(harness.accept(&envelope).is_err());
    assert_eq!(harness, before);
    assert!(harness
        .state_at(parent_hash)
        .unwrap()
        .snapshot()
        .reserve
        .unwrap()
        .shutdown
        .is_none());
}

#[test]
fn journal_trusted_tip_and_genesis_detect_valid_prefix_substitution() {
    let (state, _, _) = fixture();
    let network = Network::new_regtest(Default::default());
    let mut harness = Harness::new(&network, state.clone()).unwrap();
    let prefix = harness.journal().unwrap();
    append(&mut harness, &empty(1), 1);
    assert!(Harness::replay(
        &network,
        state.clone(),
        &prefix,
        harness.journal_anchor().unwrap()
    )
    .is_err());
    let mut bad = harness.journal().unwrap();
    bad[8] ^= 1;
    assert!(Harness::replay(
        &network,
        state.clone(),
        &bad,
        harness.journal_anchor().unwrap()
    )
    .is_err());
    let mut trailing = harness.journal().unwrap();
    trailing.push(0);
    assert!(Harness::replay(
        &network,
        state,
        &trailing,
        harness.journal_anchor().unwrap()
    )
    .is_err());
}

#[test]
fn archive_block_limit_rejects_growth_without_changing_a_branch() {
    let (state, _, _) = fixture();
    let network = Network::new_regtest(Default::default());
    let mut harness = Harness::new(&network, state.clone()).unwrap();
    let limit = u64::try_from(zsv_zebra_dev::MAX_BRANCH_BLOCKS).unwrap();
    for height in 1..=limit {
        append(&mut harness, &empty(height), height);
    }
    let parent_hash = harness.tip();
    let parent = harness.state_at(parent_hash).unwrap();
    let candidate = fixture_envelope(&parent, parent_hash, &empty(limit + 1), limit + 1).unwrap();
    let before = harness.clone();
    assert!(harness.accept(&candidate).is_err());
    assert_eq!(harness, before);
    assert_eq!(
        Harness::replay(
            &network,
            state,
            &harness.journal().unwrap(),
            harness.journal_anchor().unwrap()
        )
        .unwrap(),
        harness
    );
}
