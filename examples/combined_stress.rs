//! Deterministic signed-ledger stress cases. Prices and participation are assumptions.
use serde_json::{json, Value};
use std::{error::Error as StdError, path::PathBuf};
use zsv::*;

const Z: u64 = ZATOSHIS_PER_ZEC;
const T: u64 = MICRO_USD;

struct Fixture {
    state: State,
    borrower: Wallet,
    liquidator: Wallet,
    holder: Wallet,
    oracles: [Wallet; 3],
}

impl Fixture {
    fn new(policy: ReserveConfig, initial_cash: u64) -> Self {
        let donor = Wallet::from_seed([101; 32]);
        let borrower = Wallet::from_seed([102; 32]);
        let liquidator = Wallet::from_seed([103; 32]);
        let holder = Wallet::from_seed([104; 32]);
        let oracles = [
            Wallet::from_seed([111; 32]),
            Wallet::from_seed([112; 32]),
            Wallet::from_seed([113; 32]),
        ];
        let state = State::from_regtest_fixture_with_reserve(
            Config::regtest(oracles.each_ref().map(Wallet::public_key)),
            [
                (donor.public_key(), 1_000 * Z),
                (borrower.public_key(), 2_000 * Z),
                (liquidator.public_key(), 2_000 * Z),
                (holder.public_key(), 2_000 * Z),
            ],
            policy,
            donor.public_key(),
            initial_cash,
        )
        .unwrap();
        Self {
            state,
            borrower,
            liquidator,
            holder,
            oracles,
        }
    }

    fn report(&self, sequence: u64, price: u64, expiry: u64) -> OracleUpdate {
        OracleUpdate {
            reports: self.oracles[..2]
                .iter()
                .map(|wallet| {
                    OracleReport::sign_on(&self.state, wallet, sequence, price * T, expiry)
                })
                .collect(),
        }
    }

    fn signed(&self, wallet: &Wallet, actions: Vec<Action>) -> SignedTx {
        let nonce = self.state.snapshot().accounts[&wallet.public_key()].nonce;
        wallet.sign_on(&self.state, nonce, actions)
    }

    fn apply(&mut self, oracle: Option<OracleUpdate>, transactions: Vec<SignedTx>) {
        self.state
            .apply_block(&Block {
                height: self.state.snapshot().height + 1,
                oracle,
                transactions,
            })
            .unwrap();
        self.state.check_invariants().unwrap();
    }

    fn advance(&mut self, height: u64) {
        while self.state.snapshot().height < height {
            self.apply(None, vec![]);
        }
    }

    fn issue(
        &mut self,
        inventory: u64,
        liquidator_tokens: Option<u64>,
        debt: u64,
        collateral: u64,
    ) {
        let mut actions = vec![
            Action::Open {
                vault_id: 1,
                collateral,
            },
            Action::Borrow {
                vault_id: 1,
                amount: debt,
            },
        ];
        if inventory > 0 {
            let epoch = 1 / self.state.snapshot().reserve.unwrap().config.epoch_blocks;
            actions.push(reserve_buy(inventory, 1, epoch, 1));
        }
        let mut transactions = vec![self.signed(&self.borrower, actions)];
        if let Some(available) = liquidator_tokens {
            let mut actions = vec![
                Action::Open {
                    vault_id: 1,
                    collateral: 1_000 * Z,
                },
                Action::Borrow {
                    vault_id: 1,
                    amount: 4_000 * T,
                },
            ];
            if available < 4_000 * T {
                actions.push(Action::Transfer {
                    asset: Asset::ZUSD,
                    to: self.holder.public_key(),
                    amount: 4_000 * T - available,
                });
            }
            transactions.push(self.signed(&self.liquidator, actions));
        }
        self.apply(Some(self.report(1, 100, 100)), transactions);
    }
}

fn reserve_buy(tokens: u64, sequence: u64, epoch: u64, expiry: u64) -> Action {
    Action::ReserveBuy {
        tokens,
        oracle_sequence: sequence,
        epoch,
        valid_until: expiry,
        min_zec_out: 0,
    }
}

fn checkpoint(state: &State) -> String {
    state
        .commitment()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn usd_micro(zatoshi: u64, price_usd: u64) -> u64 {
    u64::try_from(u128::from(zatoshi) * u128::from(price_usd) * u128::from(T) / u128::from(Z))
        .unwrap()
}

fn liquidation_case(
    price: u64,
    lag: u64,
    available: u64,
    inventory: u64,
    closing_block: bool,
) -> Value {
    let mut f = Fixture::new(
        ReserveConfig {
            sunset_height: 8,
            ..ReserveConfig::default()
        },
        100 * Z,
    );
    f.issue(inventory * T, Some(available * T), 1_000 * T, 25 * Z);
    let report_height = 2 + lag;
    let liquidation_height = if closing_block { 8 } else { report_height };
    let mut outcome = "no_liquidator";
    for height in 2..=8 {
        let oracle = (height == report_height).then(|| f.report(2, price, 100));
        let mut transactions = vec![];
        let attempt = height == liquidation_height && available > 0;
        if attempt {
            transactions.push(f.signed(
                &f.liquidator,
                vec![Action::Liquidate {
                    vault_id: 1,
                    owner: f.borrower.public_key(),
                    oracle_sequence: 2,
                    max_burn: 1_000 * T,
                    min_collateral: 0,
                }],
            ));
        }
        let expected = if attempt && price >= 80 {
            outcome = "not_liquidatable";
            Some(Error::Invalid("vault not liquidatable"))
        } else if attempt && available == 100 {
            outcome = "insufficient_liquidator_tokens";
            Some(Error::Invalid("insufficient balance"))
        } else {
            if attempt {
                outcome = "liquidated";
            }
            None
        };
        if let Some(expected) = expected {
            let before = f.state.clone();
            let candidate = Block {
                height,
                oracle: oracle.clone(),
                transactions,
            };
            assert_eq!(f.state.apply_block(&candidate), Err(expected));
            assert_eq!(f.state, before);
            // The failed candidate is discarded; the same height accepts its price without the rejected order.
            f.apply(oracle, vec![]);
        } else {
            f.apply(oracle, transactions);
        }
    }
    let s = f.state.snapshot();
    let r = s.reserve.as_ref().unwrap();
    let target = s.vaults.get(&VaultId::new(f.borrower.public_key(), 1));
    let active_gap = target.map_or(0, |v| v.debt.saturating_sub(usd_micro(v.collateral, price)));
    let collateral_capacity = 25 * price * T * 100 / 105;
    let latent_before = (1_000 * T).saturating_sub(25 * price * T);
    json!({
        "true_zec_usd": price, "oracle_lag_blocks": lag,
        "liquidator_available_micro_zusd": available * T, "initial_reserve_inventory_micro_zusd": inventory * T,
        "liquidation_timing": if closing_block { "closing_block" } else { "on_report" },
        "oracle_report_height": report_height, "attempt_height": liquidation_height,
        "outcome": outcome, "recorded_bad_debt_micro_zusd": s.bad_debt,
        "shutdown_burn_micro_zusd": r.shutdown.as_ref().unwrap().bad_debt_burn,
        "remaining_reserve_inventory_micro_zusd": r.reserve_tokens,
        "active_target_principal_micro_zusd": target.map_or(0, |v| v.debt),
        "active_target_principal_shortfall_micro_usd": active_gap,
        "target_principal_shortfall_at_crash_micro_usd": latent_before,
        "target_bonus_adjusted_shortfall_at_crash_micro_zusd": (1_000 * T).saturating_sub(collateral_capacity),
        "elapsed_blocks_with_unsettled_target_since_true_price_change": if outcome == "liquidated" { liquidation_height - 2 } else { 6 },
        "stable_supply_micro_zusd": s.stable_supply,
        "nsm_zatoshi": r.nsm_zec, "final_checkpoint": checkpoint(&f.state),
    })
}

fn stale_quote_case(true_price: u64, reserve_sells: bool) -> Value {
    let mut f = Fixture::new(ReserveConfig::default(), 100 * Z);
    f.issue(
        if reserve_sells { 400 * T } else { 0 },
        None,
        1_000 * T,
        25 * Z,
    );
    f.advance(5);
    assert!(f.state.snapshot().reserve.unwrap().paused_since.is_some());
    let before = f.state.snapshot();
    let (actor, action) = if reserve_sells {
        (
            &f.holder,
            Action::ReserveSell {
                tokens: 400 * T,
                oracle_sequence: 2,
                epoch: 0,
                valid_until: 6,
                max_zec_in: 5 * Z,
            },
        )
    } else {
        (&f.borrower, reserve_buy(400 * T, 2, 0, 6))
    };
    let actor_key = actor.public_key();
    let tx = f.signed(actor, vec![action]);
    f.apply(Some(f.report(2, 100, 8)), vec![tx]);
    let after = f.state.snapshot();
    assert!(after.reserve.as_ref().unwrap().paused_since.is_none());
    let zec_flow = before.accounts[&actor_key]
        .zec
        .abs_diff(after.accounts[&actor_key].zec);
    let dollars = usd_micro(zec_flow, true_price);
    let advantage = if reserve_sells {
        i128::from(400 * T) - i128::from(dollars)
    } else {
        i128::from(dollars) - i128::from(400 * T)
    };
    json!({
        "true_zec_usd": true_price, "reported_zec_usd": 100, "resigned_old_price_at_height": 6,
        "side": if reserve_sells { "reserve_sells_tokens" } else { "reserve_buys_tokens" },
        "traded_micro_zusd": 400 * T, "zec_flow_zatoshi": zec_flow,
        "zec_flow_at_true_price_micro_usd": dollars,
        "trader_quote_advantage_if_token_worth_one_usd_micro_usd": advantage,
        "active_target_principal_shortfall_micro_usd": (1_000 * T).saturating_sub(25 * true_price * T),
        "final_checkpoint": checkpoint(&f.state),
    })
}

fn funding_case(cap_zec: u64, per_block_zec: u64, true_price: u64) -> Value {
    let mut f = Fixture::new(
        ReserveConfig {
            epoch_blocks: 1,
            funding_target_zec: 10 * Z,
            lifetime_retained_funding_cap: cap_zec * Z,
            funding_zec_per_block: per_block_zec * Z,
            funding_start_height: 1,
            funding_end_height: 11,
            sunset_height: 10,
            ..ReserveConfig::default()
        },
        10 * Z,
    );
    // First purchase is part of the same signed transaction that issues the tokens.
    f.issue(500 * T, None, 10_000 * T, 250 * Z);
    let mut fills = 1;
    let mut cash_rejections = 0;
    for height in 2..10 {
        let oracle = Some(f.report(height, 100, 100));
        let tx = f.signed(
            &f.borrower,
            vec![reserve_buy(500 * T, height, height, height)],
        );
        let candidate = Block {
            height,
            oracle: oracle.clone(),
            transactions: vec![tx],
        };
        let before = f.state.clone();
        match f.state.apply_block(&candidate) {
            Ok(()) => fills += 1,
            Err(Error::Invalid("insufficient balance")) => {
                assert_eq!(f.state, before);
                cash_rejections += 1;
                f.apply(oracle, vec![]);
            }
            other => panic!("unexpected funding-case result: {other:?}"),
        }
        f.state.check_invariants().unwrap();
    }
    f.apply(None, vec![]);
    let s = f.state.snapshot();
    let r = s.reserve.unwrap();
    json!({
        "retained_funding_cap_zatoshi": cap_zec * Z, "funding_per_block_zatoshi": per_block_zec * Z,
        "true_zec_usd": true_price, "reported_zec_usd": 100,
        "accepted_500_token_purchases": fills, "cash_rejected_purchases": cash_rejections,
        "initial_reserve_zatoshi": 10 * Z, "scheduled_funding_zatoshi": r.funded_zec,
        "retained_initial_and_new_funding_zatoshi": r.retained_funding_zec,
        "gross_purchase_payout_zatoshi": r.lifetime_zec_out, "nsm_zatoshi": r.nsm_zec,
        "unauctioned_inventory_micro_zusd": r.reserve_tokens,
        "gross_purchase_payout_at_true_price_micro_usd": usd_micro(r.lifetime_zec_out, true_price),
        "nsm_cash_at_true_price_micro_usd": usd_micro(r.nsm_zec, true_price),
        "all_public_appropriations_at_true_price_micro_usd": usd_micro(10 * Z + r.funded_zec, true_price),
        "active_target_principal_shortfall_micro_usd": (10_000 * T).saturating_sub(250 * true_price * T),
        "final_checkpoint": checkpoint(&f.state),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Participation {
    None,
    BorrowerAtFloor,
    FirstParValuingBuyer,
}

fn auction_case(true_price: u64, participation: Participation) -> Value {
    let mut f = Fixture::new(
        ReserveConfig {
            sunset_height: 3,
            ..ReserveConfig::default()
        },
        100 * Z,
    );
    let initial_borrower_zec = f.state.snapshot().accounts[&f.borrower.public_key()].zec;
    f.issue(400 * T, None, 1_000 * T, 25 * Z);
    f.advance(3);
    let mut paid = 0;
    let mut sale_height = None;
    let last_height = 2_003;
    for height in 4..=last_height {
        let r = f.state.snapshot().reserve.unwrap();
        let price = r.auction_price(height).unwrap();
        // q is 400 whole tokens here, so integer multiplication is the exact ZEC charge.
        let ask = 400 * price;
        let bid = match participation {
            Participation::None => false,
            Participation::BorrowerAtFloor => height == last_height,
            Participation::FirstParValuingBuyer => usd_micro(ask, true_price) <= 400 * T,
        };
        if bid {
            let actor = if participation == Participation::BorrowerAtFloor {
                &f.borrower
            } else {
                &f.holder
            };
            let mut actions = vec![Action::ReserveAuctionBuy {
                tokens: 400 * T,
                valid_until: height,
                max_zec_in: ask,
            }];
            if participation == Participation::BorrowerAtFloor {
                actions.extend([
                    Action::Repay {
                        vault_id: 1,
                        amount: 1_000 * T,
                    },
                    Action::Close { vault_id: 1 },
                ]);
            }
            let tx = f.signed(actor, actions);
            f.apply(None, vec![tx]);
            paid = ask;
            sale_height = Some(height);
            break;
        }
        f.apply(None, vec![]);
    }
    let s = f.state.snapshot();
    let r = s.reserve.as_ref().unwrap();
    let pnl = if participation == Participation::BorrowerAtFloor {
        assert!(!s
            .vaults
            .contains_key(&VaultId::new(f.borrower.public_key(), 1)));
        assert_eq!(s.accounts[&f.borrower.public_key()].zusd, 0);
        assert_eq!(s.stable_supply, 0);
        Some(
            i128::from(s.accounts[&f.borrower.public_key()].zec) - i128::from(initial_borrower_zec),
        )
    } else {
        None
    };
    json!({
        "true_zec_usd": true_price,
        "participation": match participation {
            Participation::None => "no_bidders",
            Participation::BorrowerAtFloor => "only_borrower_bids_at_floor",
            Participation::FirstParValuingBuyer => "par_valuing_buyer_at_first_affordable_block",
        },
        "auction_duration_blocks": 2_000, "sale_height": sale_height,
        "auction_proceeds_zatoshi": paid, "auction_proceeds_at_true_price_micro_usd": usd_micro(paid, true_price),
        "unauctioned_inventory_micro_zusd": r.reserve_tokens,
        "active_target_principal_micro_zusd": s.vaults.get(&VaultId::new(f.borrower.public_key(), 1)).map_or(0, |v| v.debt),
        "borrower_complete_cycle_pnl_zatoshi": pnl,
        "borrower_complete_cycle_pnl_at_true_price_micro_usd": pnl.map(|v| v * i128::from(true_price) * i128::from(T) / i128::from(Z)),
        "conditional_buyer_discount_to_one_usd_micro_usd": sale_height.map(|_| 400 * T - usd_micro(paid, true_price)),
        "nsm_zatoshi": r.nsm_zec, "final_checkpoint": checkpoint(&f.state),
    })
}

fn study() -> Value {
    let mut liquidation = vec![];
    for price in [10, 20, 40, 60, 80, 100] {
        for lag in [0, 2, 4] {
            for available in [0, 100, 4_000] {
                for inventory in [0, 200, 800] {
                    for closing in [false, true] {
                        liquidation
                            .push(liquidation_case(price, lag, available, inventory, closing));
                    }
                }
            }
        }
    }
    let mut stale_quotes = vec![];
    for price in [20, 100, 200] {
        for sells in [false, true] {
            stale_quotes.push(stale_quote_case(price, sells));
        }
    }
    let mut funding = vec![];
    for cap in [10, 12, 20] {
        for allocation in [0, 1] {
            for price in [20, 100] {
                funding.push(funding_case(cap, allocation, price));
            }
        }
    }
    let mut auctions = vec![];
    for price in [20, 100, 200] {
        for participation in [
            Participation::None,
            Participation::BorrowerAtFloor,
            Participation::FirstParValuingBuyer,
        ] {
            auctions.push(auction_case(price, participation));
        }
    }
    json!({
        "scope": "Deterministic signed Rust vault/reserve cases; no market calibration, stochastic probabilities, native funds or peg guarantee.",
        "units": {"zec": "zatoshi", "tokens": "micro-zUSD", "dollar_marks": "micro-USD"},
        "assumptions": {
            "liquidation": "True ZEC price changes at height 2 and stays constant. Reports arrive after 0/2/4 blocks. The target has 25 ZEC and 1000 debt; an independent liquidator has 1000 ZEC and 4000 debt. Unavailable liquidator tokens remain with a passive holder. Closure is height 8. No price-dependent new borrowing or external token trades occur.",
            "stale_quotes": "True ZEC price differs from an old $100 report. A quorum re-signs that old price at height 6 after a pause. Quote advantages assume each traded token is worth $1; they are not realized profits or claims about impaired-token value.",
            "funding": "Ten-block synthetic allocation schedule; nine attempted 500-token purchases with one-block epochs. Quoted ZEC price stays $100 while true-price marks are $20 or $100. Unsold inventory is not valued as recovered dollars.",
            "auctions": "Default 2000-block descending price, zero fees and constant true ZEC price. A par-valuing buyer is an assumed response rule, not modeled bidder competition. Borrower PnL is reported only after full debt repayment, collateral return and zero token balance.",
        },
        "case_counts": {"liquidation": liquidation.len(), "stale_quotes": stale_quotes.len(), "funding": funding.len(), "auctions": auctions.len()},
        "liquidation": liquidation, "stale_quotes": stale_quotes, "funding": funding, "auctions": auctions,
    })
}

fn main() -> Result<(), Box<dyn StdError>> {
    let output = serde_json::to_string_pretty(&study())? + "\n";
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [] => print!("{output}"),
        [check] if check == "--check" => {
            let path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("analysis/combined-results.json");
            if std::fs::read_to_string(path)? != output {
                return Err("combined stress output changed".into());
            }
            println!("Combined signed-ledger study checked: 351 deterministic cases.");
        }
        _ => return Err("usage: cargo run --locked --example combined_stress [-- --check]".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_timing_keeps_existing_and_later_bad_debt_distinct() {
        let before = liquidation_case(20, 0, 4_000, 800, false);
        let during = liquidation_case(20, 0, 4_000, 800, true);
        assert_eq!(before["recorded_bad_debt_micro_zusd"], 0);
        assert_eq!(before["shutdown_burn_micro_zusd"], 523_809_524u64);
        assert_eq!(during["recorded_bad_debt_micro_zusd"], 523_809_524u64);
        assert_eq!(during["remaining_reserve_inventory_micro_zusd"], 800 * T);
    }

    #[test]
    fn zero_recorded_bad_debt_does_not_hide_an_underwater_live_vault() {
        let case = liquidation_case(10, 4, 100, 200, false);
        assert_eq!(case["outcome"], "insufficient_liquidator_tokens");
        assert_eq!(case["recorded_bad_debt_micro_zusd"], 0);
        assert_eq!(case["active_target_principal_shortfall_micro_usd"], 750 * T);
    }

    #[test]
    fn exact_liquidation_threshold_and_bonus_shortfall_are_separate() {
        let threshold = liquidation_case(80, 0, 4_000, 0, false);
        assert_eq!(threshold["outcome"], "not_liquidatable");
        let bonus = liquidation_case(40, 0, 4_000, 0, false);
        assert_eq!(bonus["target_principal_shortfall_at_crash_micro_usd"], 0);
        assert_eq!(bonus["recorded_bad_debt_micro_zusd"], 47_619_048u64);
    }

    #[test]
    fn a_resigned_old_quote_can_pay_more_than_the_token_par_benchmark() {
        let case = stale_quote_case(200, false);
        assert_eq!(case["zec_flow_zatoshi"], 398_000_000u64);
        assert_eq!(
            case["trader_quote_advantage_if_token_worth_one_usd_micro_usd"],
            396 * T
        );
    }

    #[test]
    fn finite_allocation_does_not_guarantee_all_orders_can_fill() {
        let case = funding_case(12, 1, 20);
        assert_eq!(case["accepted_500_token_purchases"], 2);
        assert_eq!(case["cash_rejected_purchases"], 7);
        assert_eq!(case["retained_initial_and_new_funding_zatoshi"], 12 * Z);
        assert_eq!(case["scheduled_funding_zatoshi"], 10 * Z);
        assert_eq!(case["nsm_zatoshi"], 1_005_000_000u64);
    }

    #[test]
    fn floor_bid_profit_is_measured_after_debt_and_collateral_are_settled() {
        let case = auction_case(100, Participation::BorrowerAtFloor);
        assert_eq!(case["auction_proceeds_zatoshi"], 400);
        assert_eq!(case["active_target_principal_micro_zusd"], 0);
        assert_eq!(case["borrower_complete_cycle_pnl_zatoshi"], 397_999_600i64);
    }
}
