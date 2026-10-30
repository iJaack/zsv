//! Independent attacks against the collateral-backed development profile.
use zebra_chain::{
    parameters::Network,
    transaction::Transaction,
    transparent::{OutPoint, Output},
};
use zsv::{
    Action, Asset, Block, Config, OracleReport, OracleUpdate, SignedTx, State, VaultId, Wallet,
    MICRO_USD, ZATOSHIS_PER_ZEC,
};
use zsv_zebra_dev::{
    collateral::{self, CollateralHarness, CollateralState, OwnerBinding},
    collateral_wire::{self as wire, NativeIntent, NativeWallet},
};

const Z: u64 = ZATOSHIS_PER_ZEC;

struct Fixture {
    archive: CollateralHarness,
    owner: NativeWallet,
    other: NativeWallet,
    oracles: [Wallet; 3],
}
fn output(wallet: &NativeWallet, value: u64) -> Output {
    Output {
        value: value.try_into().unwrap(),
        lock_script: wire::p2pk_script(wallet.native_key()).unwrap(),
    }
}
fn fixture() -> Fixture {
    let owner = NativeWallet::from_seeds([151; 32], [161; 32]).unwrap();
    let other = NativeWallet::from_seeds([152; 32], [162; 32]).unwrap();
    let oracles = [
        Wallet::from_seed([171; 32]),
        Wallet::from_seed([172; 32]),
        Wallet::from_seed([173; 32]),
    ];
    let model = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(owner.model_key(), 20 * Z), (other.model_key(), 10 * Z)],
    )
    .unwrap();
    let bindings = [&owner, &other]
        .map(|wallet| OwnerBinding {
            owner: wallet.model_key(),
            native_key: wallet.native_key().to_vec(),
        })
        .to_vec();
    let funding =
        wire::fixture_funding(vec![output(&owner, 20 * Z), output(&other, 10 * Z)]).unwrap();
    let archive = CollateralHarness::new(
        &Network::new_regtest(Default::default()),
        model,
        bindings,
        funding,
    )
    .unwrap();
    Fixture {
        archive,
        owner,
        other,
        oracles,
    }
}
fn current(f: &Fixture) -> CollateralState {
    f.archive.state_at(f.archive.tip()).unwrap()
}
fn wallet_coin(state: &CollateralState, wallet: &NativeWallet) -> (OutPoint, Output) {
    let script = wire::p2pk_script(wallet.native_key()).unwrap();
    state
        .utxos()
        .into_iter()
        .find(|(_, output)| output.lock_script == script)
        .unwrap()
}
fn native_pair(
    state: &CollateralState,
    wallet: &NativeWallet,
    model: &zsv::SignedTx,
    inputs: &[(OutPoint, Output)],
    mut outputs: Vec<Output>,
    expiry: u32,
) -> (Transaction, NativeIntent) {
    outputs.push(Output {
        value: 0_u64.try_into().unwrap(),
        lock_script: wire::commitment_script(wire::model_commitment(model).unwrap()),
    });
    let points: Vec<_> = inputs.iter().map(|(p, _)| *p).collect();
    let previous: Vec<_> = inputs.iter().map(|(_, o)| o.clone()).collect();
    let native = wire::native_transaction(&points, &outputs, expiry).unwrap();
    let native = wire::sign_p2pk_inputs(wallet, &native, &previous).unwrap();
    let intent = wallet
        .sign_intent(state.profile_domain(), state.checkpoint(), &native, model)
        .unwrap();
    (native, intent)
}
fn transfer(f: &Fixture, amount: u64, expiry: u32) -> Vec<u8> {
    let state = current(f);
    let model_state = state.model_state();
    let nonce = model_state.snapshot().accounts[&f.owner.model_key()].nonce;
    let signed = f.owner.model_wallet().sign_on(
        &model_state,
        nonce,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.model_key(),
            amount,
        }],
    );
    let (point, coin) = wallet_coin(&state, &f.owner);
    let (native, intent) = native_pair(
        &state,
        &f.owner,
        &signed,
        &[(point, coin.clone())],
        vec![
            output(&f.owner, u64::from(coin.value) - amount),
            output(&f.other, amount),
        ],
        expiry,
    );
    collateral::fixture_envelope(
        &state,
        f.archive.tip(),
        &Block {
            height: model_state.snapshot().height + 1,
            oracle: None,
            transactions: vec![signed],
        },
        vec![native],
        vec![intent],
        1,
    )
    .unwrap()
}
fn reject(archive: &mut CollateralHarness, encoded: &[u8]) {
    let before = archive.clone();
    assert!(archive.accept(encoded).is_err());
    assert_eq!(*archive, before);
}

fn model_tx(state: &CollateralState, wallet: &NativeWallet, actions: Vec<Action>) -> SignedTx {
    let model = state.model_state();
    wallet.model_wallet().sign_on(
        &model,
        model.snapshot().accounts[&wallet.model_key()].nonce,
        actions,
    )
}
fn envelope(
    f: &Fixture,
    model: SignedTx,
    native: Transaction,
    intent: NativeIntent,
    oracle: Option<OracleUpdate>,
) -> Vec<u8> {
    let state = current(f);
    let block = Block {
        height: state.model_state().snapshot().height + 1,
        oracle,
        transactions: vec![model],
    };
    // These attacks must reach native validation with an otherwise valid model.
    state.model_state().validate_block(&block).unwrap();
    collateral::fixture_envelope(
        &state,
        f.archive.tip(),
        &block,
        vec![native],
        vec![intent],
        9,
    )
    .unwrap()
}
fn self_transfer(wallet: &NativeWallet) -> Vec<Action> {
    vec![Action::Transfer {
        asset: Asset::ZEC,
        to: wallet.model_key(),
        amount: 1,
    }]
}
fn vault_output(wallet: &NativeWallet, amount: u64) -> Output {
    Output {
        value: amount.try_into().unwrap(),
        lock_script: wire::vault_script(VaultId::new(wallet.model_key(), 1)),
    }
}
fn vault_coin(state: &CollateralState, owner: &NativeWallet) -> (OutPoint, Output) {
    state
        .utxos()
        .into_iter()
        .find(|(_, o)| o.lock_script == vault_output(owner, 0).lock_script)
        .unwrap()
}
fn oracle(f: &Fixture, state: &CollateralState, sequence: u64, price: u64) -> OracleUpdate {
    OracleUpdate {
        reports: f.oracles[..2]
            .iter()
            .map(|o| {
                OracleReport::sign_on(&state.model_state(), o, sequence, price * MICRO_USD, 100)
            })
            .collect(),
    }
}
fn open(f: &mut Fixture, send_tokens: bool) {
    let state = current(f);
    let mut actions = vec![
        Action::Open {
            vault_id: 1,
            collateral: 10 * Z,
        },
        Action::Borrow {
            vault_id: 1,
            amount: 400 * MICRO_USD,
        },
    ];
    if send_tokens {
        actions.push(Action::Transfer {
            asset: Asset::ZUSD,
            to: f.other.model_key(),
            amount: 400 * MICRO_USD,
        });
    }
    let model = model_tx(&state, &f.owner, actions);
    let pair = native_pair(
        &state,
        &f.owner,
        &model,
        &[wallet_coin(&state, &f.owner)],
        vec![output(&f.owner, 10 * Z), vault_output(&f.owner, 10 * Z)],
        100,
    );
    let encoded = envelope(f, model, pair.0, pair.1, Some(oracle(f, &state, 1, 100)));
    f.archive.accept(&encoded).unwrap();
}

// These offsets are the fixed V4 header plus one input's outpoint. Every
// mutation is deserialized again by Zebra before reaching the verifier.
fn replace_unlock(tx: &Transaction, script: &[u8]) -> Transaction {
    assert_eq!(tx.inputs().len(), 1);
    assert!(script.len() < 253);
    let mut bytes = wire::encode_transaction(tx).unwrap();
    let old_len = usize::from(bytes[45]);
    bytes.splice(46..46 + old_len, script.iter().copied());
    bytes[45] = script.len() as u8;
    wire::decode_transaction(&bytes).unwrap()
}
fn unlock(tx: &Transaction) -> Vec<u8> {
    let bytes = wire::encode_transaction(tx).unwrap();
    bytes[46..46 + usize::from(bytes[45])].to_vec()
}

#[test]
fn signed_native_expiry_is_enforced_at_the_candidate_native_height() {
    let mut f = fixture();
    // Bootstrap is native height 1, so the first child is height 2.
    let expired = transfer(&f, Z, 1);
    reject(&mut f.archive, &expired);
    let valid = transfer(&f, Z, 2);
    f.archive.accept(&valid).unwrap();
}

#[test]
fn native_signatures_bind_resolved_values_and_outputs_and_require_strict_all() {
    let f = fixture();
    let state = current(&f);
    let model = model_tx(&state, &f.owner, self_transfer(&f.owner));
    let coin = wallet_coin(&state, &f.owner);
    let (native, _) = native_pair(
        &state,
        &f.owner,
        &model,
        std::slice::from_ref(&coin),
        vec![coin.1.clone()],
        100,
    );
    wire::verify_transaction(&native, std::slice::from_ref(&coin.1)).unwrap();

    let wrong_amount = output(&f.owner, 20 * Z - 1);
    assert!(wire::verify_transaction(&native, &[wrong_amount]).is_err());
    let mut changed_outputs = native.outputs();
    changed_outputs[0] = output(&f.other, 20 * Z);
    let changed = wire::native_transaction(&[coin.0], &changed_outputs, 100).unwrap();
    let changed = replace_unlock(&changed, &unlock(&native));
    assert!(wire::verify_transaction(&changed, std::slice::from_ref(&coin.1)).is_err());

    let script = unlock(&native);
    for flag in [0, 2, 3, 0x81, 0x41] {
        let mut mutated = script.clone();
        *mutated.last_mut().unwrap() = flag;
        let changed = replace_unlock(&native, &mutated);
        assert!(
            wire::verify_transaction(&changed, std::slice::from_ref(&coin.1)).is_err(),
            "flag {flag}"
        );
    }
    let nonminimal = [vec![0x4c], script.clone()].concat();
    assert!(wire::verify_transaction(
        &replace_unlock(&native, &nonminimal),
        std::slice::from_ref(&coin.1)
    )
    .is_err());
    let mut trailing = script.clone();
    trailing.push(0);
    assert!(wire::verify_transaction(
        &replace_unlock(&native, &trailing),
        std::slice::from_ref(&coin.1)
    )
    .is_err());

    let signature = secp256k1::ecdsa::Signature::from_der(&script[1..script.len() - 1]).unwrap();
    let mut compact = signature.serialize_compact();
    let order: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x41,
    ];
    let mut borrow = 0_i16;
    for i in (0..32).rev() {
        let value = i16::from(order[i]) - i16::from(compact[32 + i]) - borrow;
        compact[32 + i] = value.rem_euclid(256) as u8;
        borrow = i16::from(value < 0);
    }
    let high_signature = secp256k1::ecdsa::Signature::from_compact(&compact).unwrap();
    assert_ne!(high_signature, signature);
    let mut normalized = high_signature;
    normalized.normalize_s();
    assert_eq!(normalized, signature);
    let high_s = high_signature.serialize_der();
    let mut high_script = vec![(high_s.len() + 1) as u8];
    high_script.extend_from_slice(&high_s);
    high_script.push(1);
    assert!(wire::verify_transaction(&replace_unlock(&native, &high_script), &[coin.1]).is_err());
}

#[test]
fn fictitious_outpoints_cannot_authorize_a_signed_deposit() {
    let mut f = fixture();
    let state = current(&f);
    let model = model_tx(
        &state,
        &f.owner,
        vec![Action::Open {
            vault_id: 1,
            collateral: 10 * Z,
        }],
    );
    let mut fake = wallet_coin(&state, &f.owner);
    fake.0.index += 100;
    let (native, intent) = native_pair(
        &state,
        &f.owner,
        &model,
        &[fake],
        vec![output(&f.owner, 10 * Z), vault_output(&f.owner, 10 * Z)],
        100,
    );
    let encoded = envelope(&f, model, native, intent, None);
    reject(&mut f.archive, &encoded);
}

#[test]
fn both_valid_signatures_do_not_allow_another_owners_native_input() {
    let mut f = fixture();
    let state = current(&f);
    let model = model_tx(&state, &f.owner, self_transfer(&f.owner));
    let coin = wallet_coin(&state, &f.other);
    let commitment = Output {
        value: 0_u64.try_into().unwrap(),
        lock_script: wire::commitment_script(wire::model_commitment(&model).unwrap()),
    };
    let native = wire::native_transaction(&[coin.0], &[coin.1.clone(), commitment], 100).unwrap();
    let native = wire::sign_p2pk_inputs(&f.other, &native, std::slice::from_ref(&coin.1)).unwrap();
    wire::verify_transaction(&native, &[coin.1]).unwrap();
    let intent = f
        .owner
        .sign_intent(state.profile_domain(), state.checkpoint(), &native, &model)
        .unwrap();
    wire::verify_intent(
        state.profile_domain(),
        state.checkpoint(),
        &native,
        &model,
        &intent,
    )
    .unwrap();
    let encoded = envelope(&f, model, native, intent, None);
    reject(&mut f.archive, &encoded);
}

#[test]
fn native_conservation_cannot_replace_per_owner_model_backing() {
    let mut f = fixture();
    let state = current(&f);
    let model = model_tx(
        &state,
        &f.owner,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.model_key(),
            amount: Z,
        }],
    );
    for (owner_value, other_value) in [(18 * Z, 2 * Z), (20 * Z, Z)] {
        let (native, intent) = native_pair(
            &state,
            &f.owner,
            &model,
            &[wallet_coin(&state, &f.owner)],
            vec![output(&f.owner, owner_value), output(&f.other, other_value)],
            100,
        );
        let encoded = envelope(&f, model.clone(), native, intent, None);
        reject(&mut f.archive, &encoded);
    }
}

#[test]
fn duplicate_inputs_and_same_block_reuse_roll_back_every_pair() {
    let mut f = fixture();
    let state = current(&f);
    let coin = wallet_coin(&state, &f.owner);
    let model = model_tx(&state, &f.owner, self_transfer(&f.owner));
    let (native, intent) = native_pair(
        &state,
        &f.owner,
        &model,
        &[coin.clone(), coin.clone()],
        vec![output(&f.owner, 40 * Z)],
        100,
    );
    let encoded = envelope(&f, model.clone(), native, intent, None);
    reject(&mut f.archive, &encoded);

    let second = f
        .owner
        .model_wallet()
        .sign_on(&state.model_state(), 1, self_transfer(&f.owner));
    let (native1, intent1) = native_pair(
        &state,
        &f.owner,
        &model,
        std::slice::from_ref(&coin),
        vec![coin.1.clone()],
        100,
    );
    let (native2, intent2) = native_pair(
        &state,
        &f.owner,
        &second,
        std::slice::from_ref(&coin),
        vec![coin.1.clone()],
        100,
    );
    let block = Block {
        height: 1,
        oracle: None,
        transactions: vec![model, second],
    };
    state.model_state().validate_block(&block).unwrap();
    let encoded = collateral::fixture_envelope(
        &state,
        f.archive.tip(),
        &block,
        vec![native1, native2],
        vec![intent1, intent2],
        11,
    )
    .unwrap();
    reject(&mut f.archive, &encoded);
}

#[test]
fn per_transaction_backing_rejects_a_block_with_matching_final_totals() {
    let mut f = fixture();
    let state = current(&f);
    let one = model_tx(
        &state,
        &f.owner,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.other.model_key(),
            amount: Z,
        }],
    );
    let two = f.other.model_wallet().sign_on(
        &state.model_state(),
        0,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.owner.model_key(),
            amount: Z,
        }],
    );
    let own_coin = wallet_coin(&state, &f.owner);
    let other_coin = wallet_coin(&state, &f.other);
    let a = native_pair(
        &state,
        &f.owner,
        &one,
        std::slice::from_ref(&own_coin),
        vec![own_coin.1.clone()],
        100,
    );
    let b = native_pair(
        &state,
        &f.other,
        &two,
        std::slice::from_ref(&other_coin),
        vec![other_coin.1.clone()],
        100,
    );
    let block = Block {
        height: 1,
        oracle: None,
        transactions: vec![one, two],
    };
    let final_model = state.model_state().validate_block(&block).unwrap();
    assert_eq!(
        final_model.snapshot().accounts[&f.owner.model_key()].zec,
        20 * Z
    );
    let encoded = collateral::fixture_envelope(
        &state,
        f.archive.tip(),
        &block,
        vec![a.0, b.0],
        vec![a.1, b.1],
        12,
    )
    .unwrap();
    reject(&mut f.archive, &encoded);
}

#[test]
fn vault_coins_cannot_rotate_under_unrelated_actions_or_unannotated_transactions() {
    let mut f = fixture();
    open(&mut f, false);
    let state = current(&f);
    let coin = vault_coin(&state, &f.owner);
    let model = model_tx(&state, &f.other, self_transfer(&f.other));
    let pair = native_pair(
        &state,
        &f.other,
        &model,
        std::slice::from_ref(&coin),
        vec![coin.1.clone()],
        100,
    );
    wire::verify_transaction(&pair.0, &[coin.1]).unwrap();
    let encoded = envelope(&f, model, pair.0.clone(), pair.1, None);
    reject(&mut f.archive, &encoded);
    let empty = Block {
        height: 2,
        oracle: None,
        transactions: vec![],
    };
    let encoded =
        collateral::fixture_envelope(&state, f.archive.tip(), &empty, vec![pair.0], vec![], 13)
            .unwrap();
    reject(&mut f.archive, &encoded);
}

#[test]
fn withdraw_and_close_cannot_redirect_or_duplicate_native_collateral() {
    let mut f = fixture();
    open(&mut f, false);
    let state = current(&f);
    let coin = vault_coin(&state, &f.owner);
    let withdrawal = model_tx(
        &state,
        &f.owner,
        vec![
            Action::Repay {
                vault_id: 1,
                amount: 100 * MICRO_USD,
            },
            Action::Withdraw {
                vault_id: 1,
                amount: Z,
            },
        ],
    );
    let pair = native_pair(
        &state,
        &f.owner,
        &withdrawal,
        std::slice::from_ref(&coin),
        vec![vault_output(&f.owner, 9 * Z), output(&f.other, Z)],
        100,
    );
    let encoded = envelope(&f, withdrawal, pair.0, pair.1, None);
    reject(&mut f.archive, &encoded);
    let close = model_tx(
        &state,
        &f.owner,
        vec![
            Action::Repay {
                vault_id: 1,
                amount: 400 * MICRO_USD,
            },
            Action::Close { vault_id: 1 },
        ],
    );
    for outputs in [
        vec![output(&f.other, 10 * Z)],
        vec![output(&f.owner, 10 * Z), vault_output(&f.owner, 10 * Z)],
    ] {
        let pair = native_pair(
            &state,
            &f.owner,
            &close,
            std::slice::from_ref(&coin),
            outputs,
            100,
        );
        let encoded = envelope(&f, close.clone(), pair.0, pair.1, None);
        reject(&mut f.archive, &encoded);
    }
    let pair = native_pair(
        &state,
        &f.owner,
        &close,
        &[coin],
        vec![output(&f.owner, 10 * Z)],
        100,
    );
    let encoded = envelope(&f, close, pair.0, pair.1, None);
    f.archive.accept(&encoded).unwrap();
    assert!(current(&f).model_state().snapshot().vaults.is_empty());
}

#[test]
fn liquidation_native_payout_must_match_both_owner_surplus_and_liquidator() {
    let mut f = fixture();
    open(&mut f, true);
    let state = current(&f);
    let coin = vault_coin(&state, &f.owner);
    let model = model_tx(
        &state,
        &f.other,
        vec![Action::Liquidate {
            vault_id: 1,
            owner: f.owner.model_key(),
            oracle_sequence: 2,
            max_burn: 400 * MICRO_USD,
            min_collateral: 1,
        }],
    );
    let report = oracle(&f, &state, 2, 50);
    let block = Block {
        height: 2,
        oracle: Some(report.clone()),
        transactions: vec![model.clone()],
    };
    let post = state
        .model_state()
        .validate_block(&block)
        .unwrap()
        .snapshot();
    let owner_payout = post.accounts[&f.owner.model_key()].zec - 10 * Z;
    let other_payout = post.accounts[&f.other.model_key()].zec - 10 * Z;
    assert!(owner_payout > 0 && other_payout > 0);
    assert_eq!(owner_payout + other_payout, 10 * Z);
    let pair = native_pair(
        &state,
        &f.other,
        &model,
        std::slice::from_ref(&coin),
        vec![output(&f.other, 10 * Z)],
        100,
    );
    let encoded = envelope(&f, model.clone(), pair.0, pair.1, Some(report.clone()));
    reject(&mut f.archive, &encoded);
    let pair = native_pair(
        &state,
        &f.other,
        &model,
        &[coin],
        vec![
            output(&f.owner, owner_payout),
            output(&f.other, other_payout),
        ],
        100,
    );
    let encoded = envelope(&f, model, pair.0, pair.1, Some(report));
    f.archive.accept(&encoded).unwrap();
}

#[test]
fn fork_replay_requires_parent_checkpoint_and_branch_local_prevouts() {
    let mut f = fixture();
    let genesis = f.archive.tip();
    let original = current(&f);
    let first = transfer(&f, Z, 100);
    let branch_a = f.archive.accept(&first).unwrap();
    let state_a = current(&f);
    let child_a = transfer(&f, Z, 100);
    f.archive.select_tip(genesis).unwrap();
    let second = transfer(&f, 2 * Z, 100);
    let branch_b = f.archive.accept(&second).unwrap();
    assert_ne!(branch_a, branch_b);
    let state_b = current(&f);
    let model = model_tx(&state_b, &f.owner, self_transfer(&f.owner));
    // Both signatures are new and valid for B, but the prevout only exists on A.
    let foreign_coin = wallet_coin(&state_a, &f.owner);
    let pair = native_pair(
        &state_b,
        &f.owner,
        &model,
        std::slice::from_ref(&foreign_coin),
        vec![foreign_coin.1.clone()],
        100,
    );
    let encoded = envelope(&f, model, pair.0, pair.1, None);
    reject(&mut f.archive, &encoded);

    // A declared native parent must be paired with its own checkpoint.
    let model = Block {
        height: 2,
        oracle: None,
        transactions: vec![],
    };
    let wrong_checkpoint =
        collateral::fixture_envelope(&state_a, branch_b, &model, vec![], vec![], 55).unwrap();
    reject(&mut f.archive, &wrong_checkpoint);
    // Valid A extension is resolved from its header even while B is selected.
    f.archive.accept(&child_a).unwrap();
    assert_eq!(f.archive.state_at(branch_b).unwrap(), state_b);
    assert_eq!(f.archive.state_at(genesis).unwrap(), original);
    assert_ne!(current(&f).checkpoint(), state_a.checkpoint());
}

#[test]
fn native_bootstrap_identity_and_owner_mapping_are_bound_to_the_profile() {
    let mut f = fixture();
    let original = current(&f);
    let owners = [&f.owner, &f.other]
        .map(|w| OwnerBinding {
            owner: w.model_key(),
            native_key: w.native_key().to_vec(),
        })
        .to_vec();
    let reversed =
        wire::fixture_funding(vec![output(&f.other, 10 * Z), output(&f.owner, 20 * Z)]).unwrap();
    let alternate = CollateralHarness::new(
        &Network::new_regtest(Default::default()),
        original.model_state(),
        owners.clone(),
        reversed,
    )
    .unwrap();
    let alternate_state = alternate.state_at(alternate.tip()).unwrap();
    assert_eq!(original.model_state(), alternate_state.model_state());
    assert_ne!(f.archive.tip(), alternate.tip());
    assert_ne!(original.profile_domain(), alternate_state.profile_domain());
    assert_ne!(original.checkpoint(), alternate_state.checkpoint());

    let model = model_tx(&original, &f.owner, self_transfer(&f.owner));
    let coin = wallet_coin(&original, &f.owner);
    let pair = native_pair(
        &original,
        &f.owner,
        &model,
        std::slice::from_ref(&coin),
        vec![coin.1.clone()],
        100,
    );
    let foreign_intent = f
        .owner
        .sign_intent(
            alternate_state.profile_domain(),
            original.checkpoint(),
            &pair.0,
            &model,
        )
        .unwrap();
    let encoded = envelope(&f, model, pair.0, foreign_intent, None);
    reject(&mut f.archive, &encoded);
    let mut aliased = owners;
    aliased[1].native_key = aliased[0].native_key.clone();
    let funding =
        wire::fixture_funding(vec![output(&f.owner, 20 * Z), output(&f.other, 10 * Z)]).unwrap();
    assert!(CollateralHarness::new(
        &Network::new_regtest(Default::default()),
        original.model_state(),
        aliased,
        funding
    )
    .is_err());
}

#[test]
fn public_networks_and_even_unfunded_reserve_genesis_are_rejected() {
    let f = fixture();
    let state = current(&f);
    let owners = [&f.owner, &f.other]
        .map(|wallet| OwnerBinding {
            owner: wallet.model_key(),
            native_key: wallet.native_key().to_vec(),
        })
        .to_vec();
    let funding =
        wire::fixture_funding(vec![output(&f.owner, 20 * Z), output(&f.other, 10 * Z)]).unwrap();
    for network in [Network::Mainnet, Network::new_default_testnet()] {
        assert!(CollateralHarness::new(
            &network,
            state.model_state(),
            owners.clone(),
            funding.clone()
        )
        .is_err());
    }
    let reserve = State::from_regtest_fixture_with_reserve(
        Config::regtest(f.oracles.each_ref().map(Wallet::public_key)),
        [(f.owner.model_key(), 20 * Z), (f.other.model_key(), 10 * Z)],
        zsv::ReserveConfig::default(),
        f.owner.model_key(),
        0,
    )
    .unwrap();
    // No initial reserve balance or future subsidy separates this from the
    // accepted vault-only balance sheet; reserve capability itself is disabled.
    assert_eq!(
        reserve.snapshot().accounts,
        state.model_state().snapshot().accounts
    );
    assert!(CollateralHarness::new(
        &Network::new_regtest(Default::default()),
        reserve,
        owners,
        funding
    )
    .is_err());
}
