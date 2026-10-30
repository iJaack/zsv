//! Fork-overlay checks for combined vault, reserve, funding and shutdown state.
#[path = "../zebra/contextual_adapter.rs"]
mod contextual_adapter;

use contextual_adapter::{commit, validate, AdapterError, Context};
use serde_json::Value;
use zsv::*;

fn before_sunset() -> (State, Wallet) {
    let document: Value =
        serde_json::from_str(include_str!("../examples/reserve-scenario-output.json")).unwrap();
    let vector = &document["vectors"][2];
    let mut state = State::from_regtest_fixture_with_reserve(
        serde_json::from_value(vector["config"].clone()).unwrap(),
        serde_json::from_value::<Vec<(Pubkey, u64)>>(vector["fixture_allocations"].clone())
            .unwrap(),
        serde_json::from_value(vector["reserve_config"].clone()).unwrap(),
        serde_json::from_value(vector["reserve_donor"].clone()).unwrap(),
        vector["initial_reserve_zec"].as_u64().unwrap(),
    )
    .unwrap();
    let blocks: Vec<Block> = serde_json::from_value(vector["blocks"].clone()).unwrap();
    for block in &blocks[..2] {
        state.apply_block(block).unwrap();
    }
    (state, Wallet::from_seed([32; 32]))
}

fn context(parent: &State) -> Context {
    Context {
        network: Network::Regtest,
        height: parent.snapshot().height + 1,
        expected_parent_commitment: parent.commitment(),
    }
}

fn sunset(parent: &State, bidder: &Wallet, actions: Vec<Action>) -> Block {
    Block {
        height: 3,
        oracle: None,
        transactions: if actions.is_empty() {
            vec![]
        } else {
            vec![bidder.sign_on(parent, 1, actions)]
        },
    }
}

fn auction() -> Action {
    Action::ReserveAuctionBuy {
        tokens: MICRO_USD,
        valid_until: 3,
        max_zec_in: ZATOSHIS_PER_ZEC,
    }
}

#[test]
fn reserve_siblings_cannot_overwrite_committed_cash_and_inventory() {
    let (mut parent, bidder) = before_sunset();
    let original = parent.clone();
    let filled = validate(
        context(&parent),
        &parent,
        &sunset(&parent, &bidder, vec![auction()]),
    )
    .unwrap();
    let empty = validate(context(&parent), &parent, &sunset(&parent, &bidder, vec![])).unwrap();
    let empty_snapshot = empty.snapshot();
    assert_eq!(parent, original);
    commit(filled, &mut parent).unwrap();
    let filled_snapshot = parent.snapshot();
    let r_filled = filled_snapshot.reserve.as_ref().unwrap();
    let r_empty = empty_snapshot.reserve.as_ref().unwrap();
    assert_eq!(r_filled.shutdown, r_empty.shutdown);
    assert_eq!(r_filled.funded_zec, r_empty.funded_zec);
    assert_eq!(r_filled.nsm_zec, r_empty.nsm_zec + ZATOSHIS_PER_ZEC);
    assert_eq!(r_filled.reserve_tokens + MICRO_USD, r_empty.reserve_tokens);
    assert_eq!(filled_snapshot.vaults, empty_snapshot.vaults);
    let committed = parent.clone();
    assert_eq!(commit(empty, &mut parent), Err(AdapterError::ParentChanged));
    assert_eq!(parent, committed);
}

#[test]
fn restoring_parent_replays_funding_shutdown_and_auction_exactly() {
    let (parent, bidder) = before_sunset();
    let block = sunset(&parent, &bidder, vec![auction()]);
    let mut branch = parent.clone();
    commit(
        validate(context(&branch), &branch, &block).unwrap(),
        &mut branch,
    )
    .unwrap();
    let expected = branch.clone();
    branch = parent.clone();
    let empty = sunset(&branch, &bidder, vec![]);
    commit(
        validate(context(&branch), &branch, &empty).unwrap(),
        &mut branch,
    )
    .unwrap();
    assert_ne!(branch, expected);
    branch = parent;
    commit(
        validate(context(&branch), &branch, &block).unwrap(),
        &mut branch,
    )
    .unwrap();
    assert_eq!(branch, expected);
    assert_eq!(branch.commitment(), expected.commitment());
    branch.check_invariants().unwrap();
}

#[test]
fn late_failure_discards_closure_funding_auction_and_nonce_together() {
    let (parent, bidder) = before_sunset();
    let before = parent.clone();
    let block = sunset(
        &parent,
        &bidder,
        vec![
            auction(),
            Action::Repay {
                vault_id: 1,
                amount: 1_000 * MICRO_USD,
            },
        ],
    );
    assert!(matches!(
        validate(context(&parent), &parent, &block),
        Err(AdapterError::Model(_))
    ));
    assert_eq!(parent, before);
    assert!(parent.snapshot().reserve.unwrap().shutdown.is_none());
}
