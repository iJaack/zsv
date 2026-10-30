//! Public fixture keys and simulated funds. No native Zcash transactions.
use serde_json::{json, Value};
use zsv::*;

#[derive(Clone, Copy)]
enum Case {
    Repay,
    BadDebt,
    Auction,
}

fn scenario(case: Case) -> Result<Value, Box<dyn std::error::Error>> {
    let donor = Wallet::from_seed([31; 32]);
    let borrower = Wallet::from_seed([32; 32]);
    let liquidator = Wallet::from_seed([33; 32]);
    let oracles = [
        Wallet::from_seed([41; 32]),
        Wallet::from_seed([42; 32]),
        Wallet::from_seed([43; 32]),
    ];
    let config = Config::regtest(oracles.each_ref().map(Wallet::public_key));
    let reserve_config = ReserveConfig {
        funding_target_zec: 20 * ZATOSHIS_PER_ZEC,
        lifetime_retained_funding_cap: 22 * ZATOSHIS_PER_ZEC,
        funding_start_height: 1,
        funding_end_height: 10,
        funding_zec_per_block: ZATOSHIS_PER_ZEC / 2,
        sunset_height: 3,
        auction_duration_blocks: 4,
        ..ReserveConfig::default()
    };
    let allocations = [
        (donor.public_key(), 100 * ZATOSHIS_PER_ZEC),
        (borrower.public_key(), 100 * ZATOSHIS_PER_ZEC),
    ];
    let seed = 20 * ZATOSHIS_PER_ZEC;
    let mut state = State::from_regtest_fixture_with_reserve(
        config.clone(),
        allocations,
        reserve_config.clone(),
        donor.public_key(),
        seed,
    )?;
    let initial = state.snapshot();
    let update = |state: &State, sequence, price| OracleUpdate {
        reports: oracles[..2]
            .iter()
            .map(|wallet| OracleReport::sign_on(state, wallet, sequence, price, 10))
            .collect(),
    };
    let inventory = if matches!(case, Case::Repay) {
        200
    } else {
        400
    } * MICRO_USD;
    let mut actions = vec![
        Action::Open {
            vault_id: 1,
            collateral: 25 * ZATOSHIS_PER_ZEC,
        },
        Action::Borrow {
            vault_id: 1,
            amount: 1_000 * MICRO_USD,
        },
        Action::ReserveBuy {
            tokens: inventory,
            oracle_sequence: 1,
            epoch: 0,
            valid_until: 1,
            min_zec_out: inventory * 995 / 1_000,
        },
    ];
    if matches!(case, Case::BadDebt) {
        actions.push(Action::Transfer {
            asset: Asset::ZUSD,
            to: liquidator.public_key(),
            amount: 600 * MICRO_USD,
        });
    }
    let first = Block {
        height: 1,
        oracle: Some(update(&state, 1, 100 * MICRO_USD)),
        transactions: vec![borrower.sign_on(&state, 0, actions)],
    };
    state.apply_block(&first)?;
    let stocked = state.snapshot();
    let second = Block {
        height: 2,
        oracle: matches!(case, Case::BadDebt).then(|| update(&state, 2, 20 * MICRO_USD)),
        transactions: match case {
            Case::Repay => vec![borrower.sign_on(
                &state,
                1,
                vec![
                    Action::ReserveSell {
                        tokens: inventory,
                        oracle_sequence: 1,
                        epoch: 0,
                        valid_until: 2,
                        max_zec_in: 201_000_000,
                    },
                    Action::Repay {
                        vault_id: 1,
                        amount: 1_000 * MICRO_USD,
                    },
                    Action::Close { vault_id: 1 },
                ],
            )],
            Case::BadDebt => vec![liquidator.sign_on(
                &state,
                0,
                vec![Action::Liquidate {
                    vault_id: 1,
                    owner: borrower.public_key(),
                    oracle_sequence: 2,
                    max_burn: 476_190_476,
                    min_collateral: 2_499_999_999,
                }],
            )],
            Case::Auction => vec![],
        },
    };
    state.apply_block(&second)?;
    let after_second = state.snapshot();
    let mut blocks = vec![first, second];
    let last_height = match case {
        Case::Repay => 2,
        Case::BadDebt => 3,
        Case::Auction => 7,
    };
    for height in 3..=last_height {
        let transactions = if matches!(case, Case::Auction) && height == last_height {
            vec![borrower.sign_on(
                &state,
                1,
                vec![
                    Action::ReserveAuctionBuy {
                        tokens: inventory,
                        valid_until: height,
                        max_zec_in: 400,
                    },
                    Action::Repay {
                        vault_id: 1,
                        amount: 1_000 * MICRO_USD,
                    },
                    Action::Close { vault_id: 1 },
                ],
            )]
        } else {
            vec![]
        };
        let block = Block {
            height,
            oracle: None,
            transactions,
        };
        state.apply_block(&block)?;
        blocks.push(block);
    }
    state.check_invariants()?;
    let final_snapshot = state.snapshot();
    let hash: String = state
        .commitment()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(json!({
        "name": match case {
            Case::Repay => "borrow_reserve_roundtrip_repay",
            Case::BadDebt => "crash_partial_bad_debt_coverage",
            Case::Auction => "uncontested_floor_auction_then_repay",
        },
        "config": config,
        "reserve_config": reserve_config,
        "fixture_allocations": allocations,
        "reserve_donor": donor.public_key(),
        "initial_reserve_zec": seed,
        "participants": {"borrower": borrower.public_key(), "liquidator": liquidator.public_key()},
        "blocks": blocks,
        "checkpoints": {"initial": initial, "stocked": stocked, "after_second": after_second, "final": final_snapshot},
        "expected_final_commitment_sha256": hash,
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = json!({
        "model": "zsv signed reserve-enabled regtest account ledger; no native consensus activation",
        "units": {"zec": "zatoshi", "zusd": "micro-zUSD", "oracle_price": "microUSD per ZEC"},
        "auction_assumption": "No competing bidders. The deliberately short four-block auction reaches its floor; this is a loss illustration, not a market forecast.",
        "vectors": [scenario(Case::Repay)?, scenario(Case::BadDebt)?, scenario(Case::Auction)?],
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
