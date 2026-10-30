//! Native wallet/vault value flows through the collateral-backed profile.
use zebra_chain::{
    parameters::Network,
    transparent::{OutPoint, Output},
};
use zsv::{
    Action, Asset, Block, Config, OracleReport, OracleUpdate, State, VaultId, Wallet, MICRO_USD,
    ZATOSHIS_PER_ZEC,
};
use zsv_zebra_dev::collateral::{self, CollateralHarness, CollateralState, OwnerBinding};
use zsv_zebra_dev::collateral_wire::{self as wire, NativeWallet, OutputKind};

const Z: u64 = ZATOSHIS_PER_ZEC;
const T: u64 = MICRO_USD;
struct Fixture {
    chain: CollateralHarness,
    alice: NativeWallet,
    bob: NativeWallet,
    oracles: [Wallet; 3],
}
fn output(amount: u64, script: zebra_chain::transparent::Script) -> Output {
    Output {
        value: amount.try_into().unwrap(),
        lock_script: script,
    }
}
fn free(amount: u64, owner: &NativeWallet) -> Output {
    output(amount, wire::p2pk_script(owner.native_key()).unwrap())
}
fn vault(amount: u64, owner: &NativeWallet) -> Output {
    output(
        amount,
        wire::vault_script(VaultId::new(owner.model_key(), 1)),
    )
}
fn fixture() -> Fixture {
    let alice = NativeWallet::from_seeds([21; 32], [31; 32]).unwrap();
    let bob = NativeWallet::from_seeds([22; 32], [32; 32]).unwrap();
    let oracles = [
        Wallet::from_seed([41; 32]),
        Wallet::from_seed([42; 32]),
        Wallet::from_seed([43; 32]),
    ];
    let state = State::from_regtest_fixture(
        Config::regtest(oracles.each_ref().map(Wallet::public_key)),
        [(alice.model_key(), 100 * Z), (bob.model_key(), 100 * Z)],
    )
    .unwrap();
    let bindings = [&alice, &bob]
        .iter()
        .map(|w| OwnerBinding {
            owner: w.model_key(),
            native_key: w.native_key().to_vec(),
        })
        .collect();
    let funding = wire::fixture_funding(vec![free(100 * Z, &alice), free(100 * Z, &bob)]).unwrap();
    let chain = CollateralHarness::new(
        &Network::new_regtest(Default::default()),
        state,
        bindings,
        funding,
    )
    .unwrap();
    Fixture {
        chain,
        alice,
        bob,
        oracles,
    }
}
fn coins(state: &CollateralState, kind: OutputKind) -> Vec<(OutPoint, Output)> {
    state
        .utxos()
        .into_iter()
        .filter(|(_, o)| wire::classify_script(&o.lock_script).unwrap() == kind)
        .collect()
}
fn report(state: &State, oracles: &[Wallet; 3], seq: u64, price: u64) -> OracleUpdate {
    OracleUpdate {
        reports: oracles[..2]
            .iter()
            .map(|o| OracleReport::sign_on(state, o, seq, price * T, 100))
            .collect(),
    }
}
fn execute(
    chain: &mut CollateralHarness,
    wallet: &NativeWallet,
    actions: Vec<Action>,
    inputs: Vec<(OutPoint, Output)>,
    mut outputs: Vec<Output>,
    oracle: Option<OracleUpdate>,
) {
    let parent = chain.state_at(chain.tip()).unwrap();
    let state = parent.model_state();
    let tx = wallet.model_wallet().sign_on(
        &state,
        state.snapshot().accounts[&wallet.model_key()].nonce,
        actions,
    );
    outputs.push(output(
        0,
        wire::commitment_script(wire::model_commitment(&tx).unwrap()),
    ));
    let native = wire::native_transaction(
        &inputs.iter().map(|(p, _)| *p).collect::<Vec<_>>(),
        &outputs,
        (state.snapshot().height + 2).try_into().unwrap(),
    )
    .unwrap();
    let native = wire::sign_p2pk_inputs(
        wallet,
        &native,
        &inputs.iter().map(|(_, o)| o.clone()).collect::<Vec<_>>(),
    )
    .unwrap();
    let intent = wallet
        .sign_intent(parent.profile_domain(), parent.checkpoint(), &native, &tx)
        .unwrap();
    let model = Block {
        height: state.snapshot().height + 1,
        oracle,
        transactions: vec![tx],
    };
    let envelope = collateral::fixture_envelope(
        &parent,
        chain.tip(),
        &model,
        vec![native],
        vec![intent],
        model.height,
    )
    .unwrap();
    chain.accept(&envelope).unwrap();
}
fn open(f: &mut Fixture) {
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    let oracle = report(&parent.model_state(), &f.oracles, 1, 100);
    execute(
        &mut f.chain,
        &f.alice,
        vec![
            Action::Open {
                vault_id: 1,
                collateral: 25 * Z,
            },
            Action::Borrow {
                vault_id: 1,
                amount: 1000 * T,
            },
        ],
        coins(&parent, OutputKind::Free(f.alice.native_key())),
        vec![free(75 * Z, &f.alice), vault(25 * Z, &f.alice)],
        Some(oracle),
    );
}

#[test]
fn deposit_withdraw_repay_and_close_move_real_outpoint_values_with_the_model() {
    let mut f = fixture();
    open(&mut f);
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    let mut inputs = coins(&parent, OutputKind::Free(f.alice.native_key()));
    inputs.extend(coins(
        &parent,
        OutputKind::Vault(VaultId::new(f.alice.model_key(), 1)),
    ));
    execute(
        &mut f.chain,
        &f.alice,
        vec![Action::Deposit {
            vault_id: 1,
            amount: 5 * Z,
        }],
        inputs,
        vec![free(70 * Z, &f.alice), vault(30 * Z, &f.alice)],
        None,
    );
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    execute(
        &mut f.chain,
        &f.alice,
        vec![Action::Withdraw {
            vault_id: 1,
            amount: 5 * Z,
        }],
        coins(
            &parent,
            OutputKind::Vault(VaultId::new(f.alice.model_key(), 1)),
        ),
        vec![free(5 * Z, &f.alice), vault(25 * Z, &f.alice)],
        None,
    );
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    execute(
        &mut f.chain,
        &f.alice,
        vec![Action::Repay {
            vault_id: 1,
            amount: 500 * T,
        }],
        coins(
            &parent,
            OutputKind::Vault(VaultId::new(f.alice.model_key(), 1)),
        ),
        vec![vault(25 * Z, &f.alice)],
        None,
    );
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    execute(
        &mut f.chain,
        &f.alice,
        vec![
            Action::Repay {
                vault_id: 1,
                amount: 500 * T,
            },
            Action::Close { vault_id: 1 },
        ],
        coins(
            &parent,
            OutputKind::Vault(VaultId::new(f.alice.model_key(), 1)),
        ),
        vec![free(25 * Z, &f.alice)],
        None,
    );
    let final_state = f.chain.state_at(f.chain.tip()).unwrap();
    let snapshot = final_state.model_state().snapshot();
    assert!(snapshot.vaults.is_empty());
    assert_eq!(snapshot.stable_supply, 0);
    assert_eq!(snapshot.accounts[&f.alice.model_key()].zec, 100 * Z);
    assert_eq!(
        coins(&final_state, OutputKind::Free(f.alice.native_key()))
            .iter()
            .map(|(_, o)| u64::from(o.value))
            .sum::<u64>(),
        100 * Z
    );
}

#[test]
fn liquidation_routes_native_collateral_to_liquidator_and_owner_surplus() {
    let mut f = fixture();
    open(&mut f);
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    execute(
        &mut f.chain,
        &f.alice,
        vec![Action::Transfer {
            asset: Asset::ZUSD,
            to: f.bob.model_key(),
            amount: 1000 * T,
        }],
        coins(&parent, OutputKind::Free(f.alice.native_key())),
        vec![free(75 * Z, &f.alice)],
        None,
    );
    let parent = f.chain.state_at(f.chain.tip()).unwrap();
    let oracle = report(&parent.model_state(), &f.oracles, 2, 20);
    execute(
        &mut f.chain,
        &f.bob,
        vec![Action::Liquidate {
            vault_id: 1,
            owner: f.alice.model_key(),
            oracle_sequence: 2,
            max_burn: 476_190_476,
            min_collateral: 2_499_999_999,
        }],
        coins(
            &parent,
            OutputKind::Vault(VaultId::new(f.alice.model_key(), 1)),
        ),
        vec![free(2_499_999_999, &f.bob), free(1, &f.alice)],
        Some(oracle),
    );
    let final_state = f.chain.state_at(f.chain.tip()).unwrap();
    let snapshot = final_state.model_state().snapshot();
    assert_eq!(snapshot.bad_debt, 523_809_524);
    assert!(snapshot.vaults.is_empty());
    assert_eq!(snapshot.accounts[&f.bob.model_key()].zec, 12_499_999_999);
    assert_eq!(snapshot.accounts[&f.alice.model_key()].zec, 7_500_000_001);
}

#[test]
fn branch_selection_restores_native_outpoints_and_model_balances_together() {
    let mut f = fixture();
    let original = f.chain.tip();
    open(&mut f);
    let opened = f.chain.tip();
    let opened_state = f.chain.state_at(opened).unwrap();
    f.chain.select_tip(original).unwrap();
    let parent = f.chain.state_at(original).unwrap();
    execute(
        &mut f.chain,
        &f.alice,
        vec![Action::Transfer {
            asset: Asset::ZEC,
            to: f.bob.model_key(),
            amount: Z,
        }],
        coins(&parent, OutputKind::Free(f.alice.native_key())),
        vec![free(99 * Z, &f.alice), free(Z, &f.bob)],
        None,
    );
    let sibling = f.chain.tip();
    assert_ne!(sibling, opened);
    assert!(f
        .chain
        .state_at(sibling)
        .unwrap()
        .model_state()
        .snapshot()
        .vaults
        .is_empty());
    assert_eq!(f.chain.state_at(opened).unwrap(), opened_state);
    f.chain.select_tip(opened).unwrap();
    assert_eq!(f.chain.state_at(f.chain.tip()).unwrap(), opened_state);
}

#[test]
fn bootstrap_total_cannot_exceed_native_money_range() {
    let f = fixture();
    let max = u64::try_from(zebra_chain::amount::MAX_MONEY).unwrap();
    let model = State::from_regtest_fixture(
        Config::regtest(f.oracles.each_ref().map(Wallet::public_key)),
        [(f.alice.model_key(), max), (f.bob.model_key(), max)],
    )
    .unwrap();
    let owners = [&f.alice, &f.bob]
        .iter()
        .map(|w| OwnerBinding {
            owner: w.model_key(),
            native_key: w.native_key().to_vec(),
        })
        .collect();
    let funding = wire::fixture_funding(vec![free(max, &f.alice), free(max, &f.bob)]).unwrap();
    assert!(CollateralHarness::new(
        &Network::new_regtest(Default::default()),
        model,
        owners,
        funding
    )
    .is_err());
}
