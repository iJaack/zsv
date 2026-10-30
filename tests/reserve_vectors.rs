//! Replay real signed vault/reserve flows from the checked-in public fixtures.
use serde_json::Value;
use zsv::*;

#[test]
fn reserve_vectors_replay_the_combined_ledger() {
    let document: Value =
        serde_json::from_str(include_str!("../examples/reserve-scenario-output.json")).unwrap();
    for vector in document["vectors"].as_array().unwrap() {
        let config: Config = serde_json::from_value(vector["config"].clone()).unwrap();
        let reserve_config: ReserveConfig =
            serde_json::from_value(vector["reserve_config"].clone()).unwrap();
        let allocations: Vec<(Pubkey, u64)> =
            serde_json::from_value(vector["fixture_allocations"].clone()).unwrap();
        let donor: Pubkey = serde_json::from_value(vector["reserve_donor"].clone()).unwrap();
        let mut state = State::from_regtest_fixture_with_reserve(
            config,
            allocations,
            reserve_config,
            donor,
            vector["initial_reserve_zec"].as_u64().unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(state.snapshot()).unwrap(),
            vector["checkpoints"]["initial"]
        );
        let blocks: Vec<Block> = serde_json::from_value(vector["blocks"].clone()).unwrap();
        for block in blocks {
            state.apply_block(&block).unwrap();
            state.check_invariants().unwrap();
            let checkpoint = match block.height {
                1 => Some("stocked"),
                2 => Some("after_second"),
                _ => None,
            };
            if let Some(checkpoint) = checkpoint {
                assert_eq!(
                    serde_json::to_value(state.snapshot()).unwrap(),
                    vector["checkpoints"][checkpoint]
                );
            }
        }
        assert_eq!(
            serde_json::to_value(state.snapshot()).unwrap(),
            vector["checkpoints"]["final"]
        );
        let hash: String = state
            .commitment()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hash, vector["expected_final_commitment_sha256"]);
    }
}
