//! Native adapters inspect detached transaction boundaries after full validation.
use zsv::*;

fn fixture(transfer: u64) -> (State, Block, Pubkey, Pubkey) {
    let owner = Wallet::from_seed([151; 32]);
    let recipient = Wallet::from_seed([152; 32]);
    let oracles = [
        Wallet::from_seed([153; 32]),
        Wallet::from_seed([154; 32]),
        Wallet::from_seed([155; 32]),
    ];
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(owner.public_key(), 10 * ZATOSHIS_PER_ZEC)],
    )
    .unwrap();
    let block = Block {
        height: 1,
        oracle: Some(OracleUpdate {
            reports: oracles[..2]
                .iter()
                .map(|key| OracleReport::sign_on(&state, key, 1, 100 * MICRO_USD, 10))
                .collect(),
        }),
        transactions: vec![
            owner.sign_on(
                &state,
                0,
                vec![
                    Action::Open {
                        vault_id: 1,
                        collateral: 5 * ZATOSHIS_PER_ZEC,
                    },
                    Action::Borrow {
                        vault_id: 1,
                        amount: 200 * MICRO_USD,
                    },
                ],
            ),
            owner.sign_on(
                &state,
                1,
                vec![Action::Transfer {
                    asset: Asset::ZUSD,
                    to: recipient.public_key(),
                    amount: transfer,
                }],
            ),
        ],
    };
    (state, block, owner.public_key(), recipient.public_key())
}

#[test]
fn trace_records_prelude_and_ordered_transactions_without_exposing_live_state() {
    let (state, block, owner, recipient) = fixture(100 * MICRO_USD);
    let original = state.clone();
    let (candidate, mut trace) = state.validate_block_with_trace(&block).unwrap();
    assert_eq!(candidate, state.validate_block(&block).unwrap());
    assert_eq!(state, original);
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].height, 1);
    assert_eq!(trace[0].oracle.as_ref().unwrap().sequence, 1);
    assert_eq!(trace[0].stable_supply, 0);
    assert_eq!(trace[0].accounts[&owner].nonce, 0);
    assert_eq!(trace[1].stable_supply, 200 * MICRO_USD);
    assert_eq!(trace[1].accounts[&owner].nonce, 1);
    assert_eq!(trace[1].accounts[&owner].zusd, 200 * MICRO_USD);
    assert!(!trace[1].accounts.contains_key(&recipient));
    assert_eq!(trace[2].accounts[&recipient].zusd, 100 * MICRO_USD);
    assert_eq!(trace[2], candidate.snapshot());
    trace[2].accounts.get_mut(&owner).unwrap().zec = u64::MAX;
    candidate.check_invariants().unwrap();
    assert_ne!(trace[2], candidate.snapshot());
}

#[test]
fn a_failed_late_transaction_returns_no_partial_trace_or_state() {
    let (state, block, _, _) = fixture(201 * MICRO_USD);
    let original = state.clone();
    assert_eq!(
        state.validate_block_with_trace(&block),
        Err(Error::Invalid("insufficient balance"))
    );
    assert_eq!(state, original);
}
