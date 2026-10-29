//! Deterministic public fixture keys only; never use these seeds to hold funds.
use serde_json::{json, Value};
use zsv::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn scenario(crash_price: u64) -> Result<Value, Box<dyn std::error::Error>> {
    let alice = Wallet::from_seed([1; 32]);
    let liquidator = Wallet::from_seed([2; 32]);
    let oracles = [
        Wallet::from_seed([11; 32]),
        Wallet::from_seed([12; 32]),
        Wallet::from_seed([13; 32]),
    ];
    let config = Config::regtest(oracles.each_ref().map(Wallet::public_key));
    let allocations = [
        (alice.public_key(), 20 * ZATOSHIS_PER_ZEC),
        (liquidator.public_key(), 20 * ZATOSHIS_PER_ZEC),
    ];
    let mut state = State::from_regtest_fixture(config.clone(), allocations)?;
    let initial = state.snapshot();
    let update = |state: &State, sequence, price| OracleUpdate {
        reports: oracles[..2]
            .iter()
            .map(|wallet| OracleReport::sign_on(state, wallet, sequence, price, 100))
            .collect(),
    };
    let open = Block {
        height: 1,
        oracle: Some(update(&state, 1, 100 * MICRO_USD)),
        transactions: vec![alice.sign_on(
            &state,
            0,
            vec![
                Action::Open {
                    vault_id: 1,
                    collateral: 10 * ZATOSHIS_PER_ZEC,
                },
                Action::Borrow {
                    vault_id: 1,
                    amount: 400 * MICRO_USD,
                },
            ],
        )],
    };
    state.apply_block(&open)?;
    let borrowed = state.snapshot();
    let (max_burn, min_collateral) = if crash_price == 50 {
        (400 * MICRO_USD, 840_000_000)
    } else {
        (190_476_190, 999_999_998)
    };
    let liquidate = Block {
        height: 2,
        oracle: Some(update(&state, 2, crash_price * MICRO_USD)),
        transactions: vec![
            alice.sign_on(
                &state,
                1,
                vec![Action::Transfer {
                    asset: Asset::ZUSD,
                    to: liquidator.public_key(),
                    amount: 400 * MICRO_USD,
                }],
            ),
            liquidator.sign_on(
                &state,
                0,
                vec![Action::Liquidate {
                    vault_id: 1,
                    owner: alice.public_key(),
                    oracle_sequence: 2,
                    max_burn,
                    min_collateral,
                }],
            ),
        ],
    };
    let independently_validated = state.validate_block(&liquidate)?;
    state.apply_block(&liquidate)?;
    assert_eq!(independently_validated, state);
    state.check_invariants()?;
    Ok(json!({
        "name": if crash_price == 50 {"solvent_full_liquidation"} else {"crash_bad_debt"},
        "config": config,
        "fixture_allocations": allocations,
        "participants": {"owner": alice.public_key(), "liquidator": liquidator.public_key()},
        "blocks": [open, liquidate],
        "checkpoints": {"initial": initial, "borrowed": borrowed, "final": state.snapshot()},
        "expected_final_commitment_sha256": hex(&state.commitment()),
        "expected": {"stable_supply": state.snapshot().stable_supply, "bad_debt": state.snapshot().bad_debt},
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = json!({
        "model": "zsv experimental regtest-only account state machine; not deployed Zcash consensus",
        "units": {"zec": "zatoshi (1e8 per ZEC)", "zusd": "microUSD (1e6 per ZUSD)", "oracle_price": "microUSD per ZEC"},
        "vectors": [scenario(50)?, scenario(20)?],
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
