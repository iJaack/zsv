use std::sync::Arc;
use zebra_chain::{
    parameters::NetworkUpgrade,
    transaction::{HashType, Transaction},
    transparent::{OutPoint, Output, Script},
};
use zsv::{Action, Config, SignedTx, State, VaultId, Wallet};
use zsv_zebra_dev::collateral_wire::{self as wire, NativeWallet, OutputKind};

fn output(value: u64, script: Script) -> Output {
    Output::new(value.try_into().unwrap(), script)
}

fn fixture() -> (NativeWallet, SignedTx, Transaction, Vec<Output>) {
    let wallet = NativeWallet::from_seeds([71; 32], [72; 32]).unwrap();
    let reporters = [73, 74, 75].map(|seed| Wallet::from_seed([seed; 32]));
    let state = State::from_regtest_fixture(
        Config::regtest(reporters.each_ref().map(Wallet::public_key)),
        [(wallet.model_key(), 100)],
    )
    .unwrap();
    let model = wallet.model_wallet().sign_on(
        &state,
        0,
        vec![Action::Open {
            vault_id: 1,
            collateral: 40,
        }],
    );
    let prevouts = vec![output(100, wire::p2pk_script(wallet.native_key()).unwrap())];
    let funding = wire::fixture_funding(prevouts.clone()).unwrap();
    let tx = wire::native_transaction(
        &[OutPoint {
            hash: funding.hash(),
            index: 0,
        }],
        &[
            output(60, wire::p2pk_script(wallet.native_key()).unwrap()),
            output(40, wire::vault_script(VaultId::new(wallet.model_key(), 1))),
            output(
                0,
                wire::commitment_script(wire::model_commitment(&model).unwrap()),
            ),
        ],
        100,
    )
    .unwrap();
    (wallet, model, tx, prevouts)
}

#[test]
fn native_v4_round_trip_and_signatures_use_the_resolved_prevout_amount() {
    let (wallet, _, unsigned, prevouts) = fixture();
    assert!(wire::verify_transaction(&unsigned, &prevouts).is_err());
    let signed = wire::sign_p2pk_inputs(&wallet, &unsigned, &prevouts).unwrap();
    wire::verify_transaction(&signed, &prevouts).unwrap();
    let bytes = wire::encode_transaction(&signed).unwrap();
    let decoded = wire::decode_transaction(&bytes).unwrap();
    assert_eq!(decoded.hash(), signed.hash());
    wire::verify_transaction(&decoded, &prevouts).unwrap();
    let mut changed = prevouts.clone();
    changed[0].value = 99_u64.try_into().unwrap();
    assert!(wire::verify_transaction(&signed, &changed).is_err());
    assert!(wire::verify_transaction(&signed, &[]).is_err());
    let other = NativeWallet::from_seeds([76; 32], [77; 32]).unwrap();
    assert!(wire::sign_p2pk_inputs(&other, &unsigned, &prevouts).is_err());
}

#[test]
fn branch_context_is_explicit_and_changes_the_native_signature_digest() {
    let (_, _, tx, prevouts) = fixture();
    let resolved = Arc::new(prevouts.clone());
    let script = prevouts[0].lock_script.as_raw_bytes().to_vec();
    let sapling = tx
        .sighash(
            NetworkUpgrade::Sapling,
            HashType::ALL,
            resolved.clone(),
            Some((0, script.clone())),
        )
        .unwrap();
    let canopy = tx
        .sighash(
            NetworkUpgrade::Canopy,
            HashType::ALL,
            resolved,
            Some((0, script)),
        )
        .unwrap();
    assert_ne!(sapling.0, canopy.0);
}

#[test]
fn intent_binds_parent_domain_outpoints_and_signed_model_but_excludes_native_signatures() {
    let (wallet, model, unsigned, prevouts) = fixture();
    let domain = [4; 32];
    let parent = [5; 32];
    let intent = wallet
        .sign_intent(domain, parent, &unsigned, &model)
        .unwrap();
    let signed = wire::sign_p2pk_inputs(&wallet, &unsigned, &prevouts).unwrap();
    wire::verify_intent(domain, parent, &signed, &model, &intent).unwrap();
    assert!(wire::verify_intent([6; 32], parent, &signed, &model, &intent).is_err());
    assert!(wire::verify_intent(domain, [6; 32], &signed, &model, &intent).is_err());
    let mut altered_model = model.clone();
    altered_model.nonce += 1;
    assert!(wire::verify_intent(domain, parent, &signed, &altered_model, &intent).is_err());
    altered_model = model.clone();
    altered_model.signature[0] ^= 1;
    assert!(wire::verify_intent(domain, parent, &signed, &altered_model, &intent).is_err());
    let mut altered_bytes = wire::encode_transaction(&signed).unwrap();
    altered_bytes[9] ^= 1; // First input's outpoint hash, after header and input count.
    let altered_native = wire::decode_transaction(&altered_bytes).unwrap();
    assert!(wire::verify_intent(domain, parent, &altered_native, &model, &intent).is_err());
}

#[test]
fn native_scripts_are_an_exact_three_class_allowlist() {
    let (wallet, _, _, _) = fixture();
    let id = VaultId::new(wallet.model_key(), 9);
    assert_eq!(
        wire::classify_script(&wire::vault_script(id)).unwrap(),
        OutputKind::Vault(id)
    );
    assert_eq!(
        wire::classify_script(&wire::p2pk_script(wallet.native_key()).unwrap()).unwrap(),
        OutputKind::Free(wallet.native_key()),
    );
    assert_eq!(
        wire::classify_script(&wire::commitment_script([3; 32])).unwrap(),
        OutputKind::Commitment([3; 32]),
    );
    for script in [vec![], vec![0x51], vec![0x6a], vec![33; 35]] {
        assert!(wire::classify_script(&Script::new(&script)).is_err());
    }
    let mut changed = wire::vault_script(id).as_raw_bytes().to_vec();
    changed[1] ^= 1;
    assert!(wire::classify_script(&Script::new(&changed)).is_err());
    assert!(wire::fixture_funding(vec![output(1, wire::vault_script(id))]).is_err());
}

#[test]
fn native_codec_rejects_trailing_bytes_and_an_empty_bundle_with_nonzero_value_balance() {
    let (wallet, model, tx, prevouts) = fixture();
    let bytes = wire::encode_transaction(&tx).unwrap();
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(wire::decode_transaction(&trailing).is_err());
    let mut nonzero = bytes;
    let balance_index = nonzero.len() - 11;
    nonzero[balance_index] = 1;
    assert!(wire::decode_transaction(&nonzero).is_err());
    for expiry in [0_u32, 500_000_000] {
        let mut altered = wire::encode_transaction(&tx).unwrap();
        let index = altered.len() - 15;
        altered[index..index + 4].copy_from_slice(&expiry.to_le_bytes());
        let invalid_expiry = wire::decode_transaction(&altered).unwrap();
        assert!(wire::verify_transaction(&invalid_expiry, &prevouts).is_err());
        assert!(wire::sign_p2pk_inputs(&wallet, &invalid_expiry, &prevouts).is_err());
        assert!(wallet
            .sign_intent([4; 32], [5; 32], &invalid_expiry, &model)
            .is_err());
    }
}

#[test]
fn vault_input_has_no_free_signature_and_requires_the_separate_stateful_guard() {
    let (wallet, model, _, _) = fixture();
    let prevout = output(40, wire::vault_script(VaultId::new(wallet.model_key(), 1)));
    let native = wire::native_transaction(
        &[OutPoint {
            hash: [9; 32].into(),
            index: 2,
        }],
        &[
            prevout.clone(),
            output(
                0,
                wire::commitment_script(wire::model_commitment(&model).unwrap()),
            ),
        ],
        100,
    )
    .unwrap();
    wire::verify_transaction(&native, &[prevout]).unwrap();
    // This success only checks the wire/crypto subset; collateral.rs must authorize
    // this vault outpoint against the signed action and model/native bucket changes.
}
