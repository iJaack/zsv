//! Exercises the proposed contextual seam against the real signed ZSV model.
#[path = "../zebra/contextual_adapter.rs"]
mod contextual_adapter;

use contextual_adapter::{commit, validate, AdapterError, Context};
use zsv::{Action, Asset, Block, Config, Error, Network, State, Wallet, ZATOSHIS_PER_ZEC};

fn fixture() -> (State, Wallet, Wallet) {
    let oracles = [
        Wallet::from_seed([1; 32]),
        Wallet::from_seed([2; 32]),
        Wallet::from_seed([3; 32]),
    ];
    let owner = Wallet::from_seed([10; 32]);
    let recipient = Wallet::from_seed([11; 32]);
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(|wallet| wallet.public_key())),
        [(owner.public_key(), 10 * ZATOSHIS_PER_ZEC)],
    )
    .unwrap();
    (state, owner, recipient)
}

fn context(state: &State) -> Context {
    Context {
        network: Network::Regtest,
        height: state.snapshot().height + 1,
        expected_parent_commitment: state.commitment(),
    }
}

fn transfer(state: &State, owner: &Wallet, recipient: &Wallet, amount: u64) -> Block {
    Block {
        height: 1,
        oracle: None,
        transactions: vec![owner.sign_on(
            state,
            0,
            vec![Action::Transfer {
                asset: Asset::ZEC,
                to: recipient.public_key(),
                amount,
            }],
        )],
    }
}

#[test]
fn preparation_is_read_only_and_commit_installs_the_verified_transition() {
    let (mut parent, owner, recipient) = fixture();
    let before = parent.clone();
    let prepared = validate(
        context(&parent),
        &parent,
        &transfer(&parent, &owner, &recipient, ZATOSHIS_PER_ZEC),
    )
    .unwrap();
    assert_eq!(parent, before);
    let mut inspected = prepared.snapshot();
    assert_eq!(inspected.height, 1);
    assert_eq!(
        inspected.accounts[&owner.public_key()].zec,
        9 * ZATOSHIS_PER_ZEC
    );
    // A review snapshot is a detached copy, not authority to edit the candidate.
    inspected.accounts.get_mut(&owner.public_key()).unwrap().zec = 0;
    commit(prepared, &mut parent).unwrap();
    let committed = parent.snapshot();
    assert_eq!(committed.height, 1);
    assert_eq!(
        committed.accounts[&owner.public_key()].zec,
        9 * ZATOSHIS_PER_ZEC
    );
    assert_eq!(
        committed.accounts[&recipient.public_key()].zec,
        ZATOSHIS_PER_ZEC
    );
    assert_eq!(committed.accounts[&owner.public_key()].nonce, 1);
}

#[test]
fn external_context_cannot_enable_mainnet_or_public_testnet() {
    let (parent, owner, recipient) = fixture();
    let before = parent.clone();
    for network in [Network::Mainnet, Network::Testnet] {
        let mut supplied = context(&parent);
        supplied.network = network;
        assert_eq!(
            validate(supplied, &parent, &transfer(&parent, &owner, &recipient, 1)).unwrap_err(),
            AdapterError::UnsupportedNetwork,
        );
    }
    assert_eq!(parent, before);
}

#[test]
fn validation_requires_the_exact_parent_checkpoint() {
    let (parent, owner, recipient) = fixture();
    let mut supplied = context(&parent);
    supplied.expected_parent_commitment[0] ^= 1;
    assert_eq!(
        validate(supplied, &parent, &transfer(&parent, &owner, &recipient, 1)).unwrap_err(),
        AdapterError::ParentChanged,
    );
}

#[test]
fn context_and_block_heights_must_both_follow_the_parent() {
    let (parent, owner, recipient) = fixture();
    for (context_height, block_height) in [(0, 0), (2, 2), (1, 2), (2, 1)] {
        let mut supplied = context(&parent);
        supplied.height = context_height;
        let mut block = transfer(&parent, &owner, &recipient, 1);
        block.height = block_height;
        assert_eq!(
            validate(supplied, &parent, &block).unwrap_err(),
            AdapterError::UnexpectedHeight,
        );
    }
}

#[test]
fn a_fresh_checkpoint_does_not_make_a_stale_height_valid() {
    let (mut parent, owner, recipient) = fixture();
    let block = transfer(&parent, &owner, &recipient, 1);
    let prepared = validate(context(&parent), &parent, &block).unwrap();
    commit(prepared, &mut parent).unwrap();
    let supplied = Context {
        network: Network::Regtest,
        height: 1,
        expected_parent_commitment: parent.commitment(),
    };
    assert_eq!(
        validate(supplied, &parent, &block).unwrap_err(),
        AdapterError::UnexpectedHeight,
    );
}

#[test]
fn sibling_preparations_cannot_overwrite_an_already_committed_child() {
    let (mut parent, owner, recipient) = fixture();
    let first = validate(
        context(&parent),
        &parent,
        &transfer(&parent, &owner, &recipient, 1),
    )
    .unwrap();
    let sibling = validate(
        context(&parent),
        &parent,
        &transfer(&parent, &owner, &recipient, 2),
    )
    .unwrap();
    commit(first, &mut parent).unwrap();
    let before = parent.clone();
    assert_eq!(
        commit(sibling, &mut parent),
        Err(AdapterError::ParentChanged)
    );
    assert_eq!(parent, before);
    assert_eq!(parent.snapshot().accounts[&recipient.public_key()].zec, 1);
}

#[test]
fn a_preparation_cannot_be_committed_onto_a_different_state_at_the_same_height() {
    let (parent, owner, recipient) = fixture();
    let prepared = validate(
        context(&parent),
        &parent,
        &transfer(&parent, &owner, &recipient, 1),
    )
    .unwrap();
    let oracles = [
        Wallet::from_seed([1; 32]),
        Wallet::from_seed([2; 32]),
        Wallet::from_seed([3; 32]),
    ];
    let mut other_parent = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(|wallet| wallet.public_key())),
        [(owner.public_key(), 11 * ZATOSHIS_PER_ZEC)],
    )
    .unwrap();
    let before = other_parent.clone();
    assert_eq!(
        commit(prepared, &mut other_parent),
        Err(AdapterError::ParentChanged)
    );
    assert_eq!(other_parent, before);
}

#[test]
fn duplicate_preparation_is_rejected_after_its_first_commit() {
    let (mut parent, owner, recipient) = fixture();
    let block = transfer(&parent, &owner, &recipient, 1);
    let first = validate(context(&parent), &parent, &block).unwrap();
    let replay = validate(context(&parent), &parent, &block).unwrap();
    commit(first, &mut parent).unwrap();
    let before = parent.clone();
    assert_eq!(
        commit(replay, &mut parent),
        Err(AdapterError::ParentChanged)
    );
    assert_eq!(parent, before);
}

#[test]
fn late_invalid_transaction_discards_the_entire_preparation() {
    let (parent, owner, recipient) = fixture();
    let before = parent.clone();
    let mut block = transfer(&parent, &owner, &recipient, 1);
    block.transactions.push(owner.sign_on(
        &parent,
        1,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: recipient.public_key(),
            amount: u64::MAX,
        }],
    ));
    assert_eq!(
        validate(context(&parent), &parent, &block).unwrap_err(),
        AdapterError::Model(Error::Invalid("insufficient balance")),
    );
    assert_eq!(parent, before);
}

#[test]
fn preparation_verifies_real_signatures_before_exposing_a_candidate() {
    let (parent, owner, recipient) = fixture();
    let before = parent.clone();
    let mut block = transfer(&parent, &owner, &recipient, 1);
    block.transactions[0].signature[0] ^= 1;
    assert_eq!(
        validate(context(&parent), &parent, &block).unwrap_err(),
        AdapterError::Model(Error::Invalid("invalid signature")),
    );
    assert_eq!(parent, before);
}
