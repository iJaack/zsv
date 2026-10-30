//! Independent adversarial probes for the bounded Zebra-linked development harness.
use std::sync::Arc;
use zebra_chain::{
    block,
    parameters::Network,
    serialization::{ZcashDeserialize, ZcashSerialize},
    transaction::Transaction,
};
use zsv::{Action, Asset, Block, Config, State, Wallet, ZATOSHIS_PER_ZEC};
use zsv_zebra_dev::{
    fixture_envelope, Error, Harness, MAX_ENVELOPE_BYTES, MAX_NATIVE_BYTES, MAX_PAYLOAD_BYTES,
};

fn fixture() -> (State, Wallet, Wallet, Network) {
    let owner = Wallet::from_seed([201; 32]);
    let recipient = Wallet::from_seed([202; 32]);
    let oracles = [
        Wallet::from_seed([211; 32]),
        Wallet::from_seed([212; 32]),
        Wallet::from_seed([213; 32]),
    ];
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), 20 * ZATOSHIS_PER_ZEC)],
    )
    .unwrap();
    (
        state,
        owner,
        recipient,
        Network::new_regtest(Default::default()),
    )
}
fn transfer(state: &State, owner: &Wallet, recipient: &Wallet, amount: u64) -> Block {
    Block {
        height: state.snapshot().height + 1,
        oracle: None,
        transactions: vec![owner.sign_on(
            state,
            state.snapshot().accounts[&owner.public_key()].nonce,
            vec![Action::Transfer {
                asset: Asset::ZEC,
                to: recipient.public_key(),
                amount,
            }],
        )],
    }
}
fn reject(archive: &mut Harness, encoded: &[u8]) -> Error {
    let before = archive.clone();
    let error = archive.accept(encoded).unwrap_err();
    assert_eq!(*archive, before);
    error
}
fn parts(encoded: &[u8]) -> (&[u8], &[u8]) {
    let n = u32::from_le_bytes(encoded[40..44].try_into().unwrap()) as usize;
    let p = 44 + n;
    let m = u32::from_le_bytes(encoded[p..p + 4].try_into().unwrap()) as usize;
    (&encoded[44..p], &encoded[p + 4..p + 4 + m])
}
fn frame(original: &[u8], native: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut encoded = original[..40].to_vec();
    encoded.extend_from_slice(&u32::try_from(native.len()).unwrap().to_le_bytes());
    encoded.extend_from_slice(native);
    encoded.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
    encoded.extend_from_slice(payload);
    encoded
}
fn change_native(encoded: &[u8], edit: impl FnOnce(&mut block::Block)) -> Vec<u8> {
    let (native, payload) = parts(encoded);
    let mut block = block::Block::zcash_deserialize(native).unwrap();
    edit(&mut block);
    let mut bytes = vec![];
    block.zcash_serialize(&mut bytes).unwrap();
    frame(encoded, &bytes, payload)
}
fn change_coinbase(block: &mut block::Block, edit: impl FnOnce(&mut Vec<u8>), fix_merkle: bool) {
    let mut bytes = vec![];
    block.transactions[0].zcash_serialize(&mut bytes).unwrap();
    edit(&mut bytes);
    block.transactions[0] = Arc::new(Transaction::zcash_deserialize(bytes.as_slice()).unwrap());
    if fix_merkle {
        Arc::make_mut(&mut block.header).merkle_root = block.transactions.iter().collect();
    }
}

#[test]
fn public_networks_and_non_genesis_model_states_cannot_initialize_the_harness() {
    let (mut state, owner, recipient, network) = fixture();
    for public in [Network::Mainnet, Network::new_default_testnet()] {
        assert!(Harness::new(&public, state.clone()).is_err());
    }
    state
        .apply_block(&transfer(&state, &owner, &recipient, 1))
        .unwrap();
    assert!(Harness::new(&network, state).is_err());
}

#[test]
fn canonical_valid_model_payload_cannot_replace_the_native_committed_payload() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let one = transfer(&state, &owner, &recipient, 1);
    let two = transfer(&state, &owner, &recipient, 2);
    let original = fixture_envelope(&state, archive.tip(), &one, 1).unwrap();
    let changed = frame(
        &original,
        parts(&original).0,
        &serde_json::to_vec(&two).unwrap(),
    );
    assert_eq!(
        reject(&mut archive, &changed),
        Error::Invalid("unique zero-value coinbase commitment required")
    );
    archive.accept(&original).unwrap();
}

#[test]
fn coinbase_bytes_require_both_merkle_binding_and_the_correct_payload_digest() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let original = fixture_envelope(
        &state,
        archive.tip(),
        &transfer(&state, &owner, &recipient, 1),
        1,
    )
    .unwrap();
    for fix_merkle in [false, true] {
        let changed = change_native(&original, |block| {
            change_coinbase(
                block,
                |bytes| {
                    let at = bytes
                        .windows(8)
                        .position(|part| part == b"ZSVDEV01")
                        .unwrap();
                    bytes[at + 8] ^= 1;
                },
                fix_merkle,
            )
        });
        let expected = if fix_merkle {
            "unique zero-value coinbase commitment required"
        } else {
            "transaction merkle root"
        };
        assert_eq!(reject(&mut archive, &changed), Error::Invalid(expected));
    }
}

#[test]
fn native_commitment_does_not_substitute_for_model_signature_validation() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let mut model = transfer(&state, &owner, &recipient, 1);
    model.transactions[0].signature[0] ^= 1;
    let encoded = fixture_envelope(&state, archive.tip(), &model, 1).unwrap();
    assert_eq!(
        reject(&mut archive, &encoded),
        Error::Model(zsv::Error::Invalid("invalid signature"))
    );
}

#[test]
fn native_coinbase_height_must_match_the_committed_model_height() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let original = fixture_envelope(
        &state,
        archive.tip(),
        &transfer(&state, &owner, &recipient, 1),
        1,
    )
    .unwrap();
    let changed = change_native(&original, |block| {
        change_coinbase(
            block,
            |bytes| {
                // V1 + one input + outpoint + short script length; this fixture starts with OP_1.
                assert_eq!(bytes[42], 0x51);
                bytes[42] = 0x52;
            },
            true,
        )
    });
    assert_eq!(
        reject(&mut archive, &changed),
        Error::Invalid("native and model heights differ")
    );
}

#[test]
fn byte_limits_and_canonical_payload_checks_precede_state_changes() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let original = fixture_envelope(
        &state,
        archive.tip(),
        &transfer(&state, &owner, &recipient, 1),
        1,
    )
    .unwrap();
    assert_eq!(
        reject(&mut archive, &vec![0; MAX_ENVELOPE_BYTES + 1]),
        Error::Invalid("envelope byte limit")
    );
    let mut oversized_native = original.clone();
    oversized_native[40..44]
        .copy_from_slice(&u32::try_from(MAX_NATIVE_BYTES + 1).unwrap().to_le_bytes());
    assert_eq!(
        reject(&mut archive, &oversized_native),
        Error::Invalid("native byte limit")
    );
    let (native, payload) = parts(&original);
    let mut oversized_payload = original.clone();
    let at = 44 + native.len();
    oversized_payload[at..at + 4]
        .copy_from_slice(&u32::try_from(MAX_PAYLOAD_BYTES + 1).unwrap().to_le_bytes());
    assert_eq!(
        reject(&mut archive, &oversized_payload),
        Error::Invalid("payload byte limit")
    );
    let mut whitespace = vec![b' '];
    whitespace.extend_from_slice(payload);
    assert_eq!(
        reject(&mut archive, &frame(&original, native, &whitespace)),
        Error::Invalid("noncanonical model payload")
    );
    let mut trailing = original.clone();
    trailing.push(0);
    assert_eq!(
        reject(&mut archive, &trailing),
        Error::Invalid("trailing envelope bytes")
    );
    reject(&mut archive, &original[..original.len() - 1]);
}

#[test]
fn native_parent_and_model_checkpoint_select_the_same_exact_branch() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let anchor = archive.tip();
    let a = archive
        .accept(
            &fixture_envelope(&state, anchor, &transfer(&state, &owner, &recipient, 10), 1)
                .unwrap(),
        )
        .unwrap();
    let b = archive
        .accept(
            &fixture_envelope(&state, anchor, &transfer(&state, &owner, &recipient, 20), 2)
                .unwrap(),
        )
        .unwrap();
    let parent_a = archive.state_at(a).unwrap();
    let child = transfer(&parent_a, &owner, &recipient, 3);
    let wrong_branch = fixture_envelope(&parent_a, b, &child, 3).unwrap();
    assert_eq!(
        reject(&mut archive, &wrong_branch),
        Error::Invalid("parent model checkpoint")
    );
    let accepted = archive
        .accept(&fixture_envelope(&parent_a, a, &child, 3).unwrap())
        .unwrap();
    assert_eq!(
        archive.state_at(accepted).unwrap().snapshot().accounts[&recipient.public_key()].zec,
        13
    );
    assert_eq!(
        archive.state_at(b).unwrap().snapshot().accounts[&recipient.public_key()].zec,
        20
    );
    let mut detached = archive.state_at(a).unwrap();
    detached.apply_block(&child).unwrap();
    assert_eq!(archive.state_at(a).unwrap(), parent_a);
}

#[test]
fn duplicate_blocks_unknown_parents_and_rewritten_checkpoints_are_atomic_rejections() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let model = transfer(&state, &owner, &recipient, 1);
    let original = fixture_envelope(&state, archive.tip(), &model, 1).unwrap();
    let unknown = fixture_envelope(&state, block::Hash([0xff; 32]), &model, 2).unwrap();
    assert_eq!(
        reject(&mut archive, &unknown),
        Error::Invalid("unknown native parent")
    );
    let mut rewritten = original.clone();
    rewritten[8] ^= 1;
    assert_eq!(
        reject(&mut archive, &rewritten),
        Error::Invalid("parent model checkpoint")
    );
    archive.accept(&original).unwrap();
    assert_eq!(
        reject(&mut archive, &original),
        Error::Invalid("duplicate native block")
    );
}

#[test]
fn trusted_archive_digest_detects_dropped_siblings_even_if_selected_tip_survives() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let genesis = archive.tip();
    let a = archive
        .accept(
            &fixture_envelope(&state, genesis, &transfer(&state, &owner, &recipient, 1), 1)
                .unwrap(),
        )
        .unwrap();
    archive
        .accept(
            &fixture_envelope(&state, genesis, &transfer(&state, &owner, &recipient, 2), 2)
                .unwrap(),
        )
        .unwrap();
    archive.select_tip(a).unwrap();
    let expected = archive.journal_anchor().unwrap();
    let original = archive.journal().unwrap();
    let first_length = u32::from_le_bytes(original[108..112].try_into().unwrap()) as usize;
    let mut truncated = original[..112 + first_length].to_vec();
    truncated[104..108].copy_from_slice(&1u32.to_le_bytes());
    assert!(Harness::replay(&network, state.clone(), &truncated, expected).is_err());
    let restored = Harness::replay(&network, state, &original, expected).unwrap();
    assert_eq!(restored, archive);
}

#[test]
fn journal_replay_requires_the_original_configuration_and_external_anchor() {
    let (state, owner, recipient, network) = fixture();
    let mut archive = Harness::new(&network, state.clone()).unwrap();
    let model = transfer(&state, &owner, &recipient, 1);
    archive
        .accept(&fixture_envelope(&state, archive.tip(), &model, 1).unwrap())
        .unwrap();
    let expected = archive.journal_anchor().unwrap();
    let journal = archive.journal().unwrap();
    let oracles = [
        Wallet::from_seed([211; 32]),
        Wallet::from_seed([212; 32]),
        Wallet::from_seed([213; 32]),
    ];
    let different_genesis = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), 21 * ZATOSHIS_PER_ZEC)],
    )
    .unwrap();
    assert!(Harness::replay(&network, different_genesis, &journal, expected).is_err());
    let mut wrong_tip = expected;
    wrong_tip.tip = block::Hash([0x77; 32]);
    assert!(Harness::replay(&network, state.clone(), &journal, wrong_tip).is_err());
    let mut wrong_digest = expected;
    wrong_digest.sha256[0] ^= 1;
    assert!(Harness::replay(&network, state.clone(), &journal, wrong_digest).is_err());
    let mut corrupt = journal.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(Harness::replay(&network, state, &corrupt, expected).is_err());
}
