//! Replay the checked-in signed, machine-readable scenario vectors.
use serde_json::Value;
use zsv::*;

#[test]
fn signed_vectors_replay_to_their_expected_checkpoints() {
    let document: Value =
        serde_json::from_str(include_str!("../examples/scenario-output.json")).unwrap();
    for vector in document["vectors"].as_array().unwrap() {
        let config: Config = serde_json::from_value(vector["config"].clone()).unwrap();
        let allocations: Vec<(Pubkey, u64)> =
            serde_json::from_value(vector["fixture_allocations"].clone()).unwrap();
        let blocks: Vec<Block> = serde_json::from_value(vector["blocks"].clone()).unwrap();
        let mut state = State::from_regtest_fixture(config, allocations).unwrap();
        assert_eq!(
            serde_json::to_value(state.snapshot()).unwrap(),
            vector["checkpoints"]["initial"]
        );
        for block in &blocks {
            state.apply_block(block).unwrap();
            state.check_invariants().unwrap();
        }
        assert_eq!(
            serde_json::to_value(state.snapshot()).unwrap(),
            vector["checkpoints"]["final"]
        );
        let hash: String = state
            .commitment()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hash,
            vector["expected_final_commitment_sha256"].as_str().unwrap()
        );
    }
}
