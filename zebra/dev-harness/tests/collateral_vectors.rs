//! Replay native transaction bytes and signed model intents from the public fixtures.
use serde_json::Value;
use zebra_chain::parameters::Network;
use zsv::{Config, Pubkey, State, ZATOSHIS_PER_ZEC};
use zsv_zebra_dev::{
    collateral::{CollateralHarness, OwnerBinding},
    collateral_wire,
};

fn unhex(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    assert!(text.len().is_multiple_of(2));
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn collateral_vectors_replay_native_backing_and_the_signed_ledger() {
    let document: Value =
        serde_json::from_str(include_str!("../collateral-scenario-output.json")).unwrap();
    for vector in document["scenarios"].as_array().unwrap() {
        let config: Config = serde_json::from_value(vector["config"].clone()).unwrap();
        let allocations: Vec<(Pubkey, u64)> =
            serde_json::from_value(vector["fixture_allocations"].clone()).unwrap();
        let owners: Vec<OwnerBinding> = serde_json::from_value(vector["owners"].clone()).unwrap();
        let funding =
            collateral_wire::decode_transaction(&unhex(&vector["native_funding_transaction_hex"]))
                .unwrap();
        let genesis = State::from_regtest_fixture(config, allocations).unwrap();
        let mut chain = CollateralHarness::new(
            &Network::new_regtest(Default::default()),
            genesis,
            owners,
            funding,
        )
        .unwrap();
        assert_eq!(chain.tip().to_string(), vector["bootstrap_native_hash"]);
        let initial = chain.state_at(chain.tip()).unwrap();
        assert_eq!(hex(&initial.profile_domain()), vector["profile_domain"]);
        assert_eq!(
            hex(&initial.checkpoint()),
            vector["initial"]["profile_checkpoint"]
        );
        for (index, envelope) in vector["envelopes_hex"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            chain.accept(&unhex(envelope)).unwrap();
            let state = chain.state_at(chain.tip()).unwrap();
            state.model_state().check_invariants().unwrap();
            assert_eq!(
                state
                    .utxos()
                    .iter()
                    .map(|(_, output)| u64::from(output.value))
                    .sum::<u64>(),
                40 * ZATOSHIS_PER_ZEC
            );
            if index == 0 {
                assert_eq!(
                    hex(&state.checkpoint()),
                    vector["opened"]["profile_checkpoint"]
                );
            }
        }
        let final_state = chain.state_at(chain.tip()).unwrap();
        assert_eq!(chain.tip().to_string(), vector["final_native_hash"]);
        assert_eq!(
            hex(&final_state.checkpoint()),
            vector["final"]["profile_checkpoint"]
        );
        assert_eq!(
            serde_json::to_value(final_state.model_state().snapshot()).unwrap(),
            vector["final"]["model"]
        );
    }
}
