//! Reproduce the signed reserve scenarios through real Zebra block commitments.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zebra_chain::parameters::Network;
use zsv::{Block, Pubkey, State};
use zsv_zebra_dev::{fixture_envelope, Harness, SOURCE_PIN};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document: Value = serde_json::from_str(include_str!(
        "../../../../examples/reserve-scenario-output.json"
    ))?;
    let network = Network::new_regtest(Default::default());
    let mut summaries = Vec::new();
    for vector in document["vectors"].as_array().ok_or("vectors missing")? {
        let genesis = State::from_regtest_fixture_with_reserve(
            serde_json::from_value(vector["config"].clone())?,
            serde_json::from_value::<Vec<(Pubkey, u64)>>(vector["fixture_allocations"].clone())?,
            serde_json::from_value(vector["reserve_config"].clone())?,
            serde_json::from_value(vector["reserve_donor"].clone())?,
            vector["initial_reserve_zec"]
                .as_u64()
                .ok_or("seed missing")?,
        )?;
        let mut harness = Harness::new(&network, genesis.clone())?;
        let blocks: Vec<Block> = serde_json::from_value(vector["blocks"].clone())?;
        for block in &blocks {
            let parent_hash = harness.tip();
            let parent = harness.state_at(parent_hash).ok_or("parent missing")?;
            harness.accept(&fixture_envelope(
                &parent,
                parent_hash,
                block,
                block.height,
            )?)?;
        }
        let final_state = harness.state_at(harness.tip()).ok_or("tip missing")?;
        assert_eq!(
            serde_json::to_value(final_state.snapshot())?,
            vector["checkpoints"]["final"]
        );
        let journal = harness.journal()?;
        let replayed = Harness::replay(&network, genesis, &journal, harness.journal_anchor()?)?;
        assert_eq!(harness, replayed);
        summaries.push(json!({
            "name": vector["name"],
            "blocks": blocks.len(),
            "native_tip": harness.tip().to_string(),
            "model_checkpoint_sha256": hex(&final_state.commitment()),
            "journal_bytes": journal.len(),
            "journal_sha256": hex(&Sha256::digest(&journal)),
            "replay_equal": true,
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "zebra_source_pin": SOURCE_PIN,
            "scope": "Offline V1 coinbase data commitments; no PoW, native collateral or native stablecoin enforcement",
            "scenarios": summaries,
        }))?
    );
    Ok(())
}
