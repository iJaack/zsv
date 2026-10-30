//! Public fixture keys and fictional native allocations, never a live deposit flow.
use serde_json::{json, Value};
use zebra_chain::{parameters::Network, transparent::Output};
use zsv::*;
use zsv_zebra_dev::{
    collateral::{fixture_envelope, CollateralHarness, CollateralState, OwnerBinding},
    collateral_wire::{self as wire, NativeWallet, OutputKind},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn output(value: u64, kind: OutputKind) -> Result<Output> {
    let script = match kind {
        OutputKind::Free(key) => wire::p2pk_script(key)?,
        OutputKind::Vault(id) => wire::vault_script(id),
        OutputKind::Commitment(digest) => wire::commitment_script(digest),
    };
    Ok(Output::new(value.try_into()?, script))
}
fn native_pair(
    parent: &CollateralState,
    wallet: &NativeWallet,
    model: &SignedTx,
    spend: OutputKind,
    mut outputs: Vec<Output>,
) -> Result<(zebra_chain::transaction::Transaction, wire::NativeIntent)> {
    let coins: Vec<_> = parent
        .utxos()
        .into_iter()
        .filter(|(_, output)| {
            wire::classify_script(&output.lock_script).is_ok_and(|kind| kind == spend)
        })
        .collect();
    assert!(!coins.is_empty());
    let points: Vec<_> = coins.iter().map(|(point, _)| *point).collect();
    let previous: Vec<_> = coins.into_iter().map(|(_, output)| output).collect();
    outputs.push(output(
        0,
        OutputKind::Commitment(wire::model_commitment(model)?),
    )?);
    let unsigned = wire::native_transaction(&points, &outputs, 100)?;
    let native = wire::sign_p2pk_inputs(wallet, &unsigned, &previous)?;
    let intent =
        wallet.sign_intent(parent.profile_domain(), parent.checkpoint(), &native, model)?;
    Ok((native, intent))
}
fn state_summary(state: &CollateralState) -> Value {
    json!({
        "profile_checkpoint": hex(&state.checkpoint()),
        "model_checkpoint": hex(&state.model_state().commitment()),
        "model": state.model_state().snapshot(),
        "utxos": state.utxos().into_iter().map(|(point, output)| json!({
            "txid": point.hash.to_string(), "index": point.index,
            "zatoshi": u64::from(output.value), "script_hex": hex(output.lock_script.as_raw_bytes()),
        })).collect::<Vec<_>>(),
    })
}

fn scenario(crash_price: Option<u64>) -> Result<Value> {
    let owner = NativeWallet::from_seeds([21; 32], [31; 32])?;
    let liquidator = NativeWallet::from_seeds([22; 32], [32; 32])?;
    let oracles = [
        Wallet::from_seed([41; 32]),
        Wallet::from_seed([42; 32]),
        Wallet::from_seed([43; 32]),
    ];
    let config = Config::regtest(oracles.each_ref().map(Wallet::public_key));
    let allocations = [
        (owner.model_key(), 20 * ZATOSHIS_PER_ZEC),
        (liquidator.model_key(), 20 * ZATOSHIS_PER_ZEC),
    ];
    let genesis = State::from_regtest_fixture(config.clone(), allocations)?;
    let bindings = vec![
        OwnerBinding {
            owner: owner.model_key(),
            native_key: owner.native_key().to_vec(),
        },
        OwnerBinding {
            owner: liquidator.model_key(),
            native_key: liquidator.native_key().to_vec(),
        },
    ];
    let funding = wire::fixture_funding(vec![
        output(20 * ZATOSHIS_PER_ZEC, OutputKind::Free(owner.native_key()))?,
        output(
            20 * ZATOSHIS_PER_ZEC,
            OutputKind::Free(liquidator.native_key()),
        )?,
    ])?;
    let funding_hex = hex(&wire::encode_transaction(&funding)?);
    let network = Network::new_regtest(Default::default());
    let mut chain =
        CollateralHarness::new(&network, genesis.clone(), bindings.clone(), funding.clone())?;
    let bootstrap = chain.tip();
    let initial = chain.state_at(bootstrap).unwrap();
    let oracle = |state: &State, sequence, price| OracleUpdate {
        reports: oracles[..2]
            .iter()
            .map(|key| OracleReport::sign_on(state, key, sequence, price * MICRO_USD, 100))
            .collect(),
    };
    let mut envelopes = vec![];
    let vault = VaultId::new(owner.model_key(), 1);
    let parent = chain.state_at(chain.tip()).unwrap();
    let model_state = parent.model_state();
    let request = owner.model_wallet().sign_on(
        &model_state,
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
    );
    let (native, intent) = native_pair(
        &parent,
        &owner,
        &request,
        OutputKind::Free(owner.native_key()),
        vec![
            output(10 * ZATOSHIS_PER_ZEC, OutputKind::Free(owner.native_key()))?,
            output(10 * ZATOSHIS_PER_ZEC, OutputKind::Vault(vault))?,
        ],
    )?;
    let block = Block {
        height: 1,
        oracle: Some(oracle(&model_state, 1, 100)),
        transactions: vec![request],
    };
    let envelope = fixture_envelope(&parent, chain.tip(), &block, vec![native], vec![intent], 1)?;
    chain.accept(&envelope)?;
    envelopes.push(envelope);
    let opened = state_summary(&chain.state_at(chain.tip()).unwrap());

    let parent = chain.state_at(chain.tip()).unwrap();
    let state = parent.model_state();
    let (actions, spend, outputs) = if crash_price.is_some() {
        (
            vec![Action::Transfer {
                asset: Asset::ZUSD,
                to: liquidator.model_key(),
                amount: 400 * MICRO_USD,
            }],
            OutputKind::Free(owner.native_key()),
            vec![output(
                10 * ZATOSHIS_PER_ZEC,
                OutputKind::Free(owner.native_key()),
            )?],
        )
    } else {
        (
            vec![
                Action::Repay {
                    vault_id: 1,
                    amount: 400 * MICRO_USD,
                },
                Action::Close { vault_id: 1 },
            ],
            OutputKind::Vault(vault),
            vec![output(
                10 * ZATOSHIS_PER_ZEC,
                OutputKind::Free(owner.native_key()),
            )?],
        )
    };
    let request = owner.model_wallet().sign_on(&state, 1, actions);
    let (native, intent) = native_pair(&parent, &owner, &request, spend, outputs)?;
    let block = Block {
        height: 2,
        oracle: None,
        transactions: vec![request],
    };
    let envelope = fixture_envelope(&parent, chain.tip(), &block, vec![native], vec![intent], 2)?;
    chain.accept(&envelope)?;
    envelopes.push(envelope);

    if let Some(price) = crash_price {
        let parent = chain.state_at(chain.tip()).unwrap();
        let state = parent.model_state();
        let (burn, seized) = if price == 50 {
            (400 * MICRO_USD, 840_000_000)
        } else {
            (190_476_190, 999_999_998)
        };
        let request = liquidator.model_wallet().sign_on(
            &state,
            0,
            vec![Action::Liquidate {
                vault_id: 1,
                owner: owner.model_key(),
                oracle_sequence: 2,
                max_burn: burn,
                min_collateral: seized,
            }],
        );
        let (native, intent) = native_pair(
            &parent,
            &liquidator,
            &request,
            OutputKind::Vault(vault),
            vec![
                output(seized, OutputKind::Free(liquidator.native_key()))?,
                output(
                    10 * ZATOSHIS_PER_ZEC - seized,
                    OutputKind::Free(owner.native_key()),
                )?,
            ],
        )?;
        let block = Block {
            height: 3,
            oracle: Some(oracle(&state, 2, price)),
            transactions: vec![request],
        };
        let envelope =
            fixture_envelope(&parent, chain.tip(), &block, vec![native], vec![intent], 3)?;
        chain.accept(&envelope)?;
        envelopes.push(envelope);
    }
    let mut replay = CollateralHarness::new(&network, genesis, bindings.clone(), funding)?;
    for envelope in &envelopes {
        replay.accept(envelope)?;
    }
    assert_eq!(chain, replay);
    let final_state = chain.state_at(chain.tip()).unwrap();
    assert_eq!(
        final_state
            .utxos()
            .iter()
            .map(|(_, output)| u64::from(output.value))
            .sum::<u64>(),
        40 * ZATOSHIS_PER_ZEC
    );
    Ok(json!({
        "name": match crash_price { None => "native_collateral_repay_and_close", Some(50) => "native_collateral_solvent_liquidation", _ => "native_collateral_crash_bad_debt" },
        "config": config, "fixture_allocations": allocations, "owners": bindings,
        "native_funding_transaction_hex": funding_hex, "bootstrap_native_hash": bootstrap.to_string(),
        "profile_domain": hex(&initial.profile_domain()), "initial": state_summary(&initial),
        "opened": opened, "envelopes_hex": envelopes.iter().map(|bytes| hex(bytes)).collect::<Vec<_>>(),
        "final_native_hash": chain.tip().to_string(), "final": state_summary(&final_state), "replay_equal": true,
    }))
}

fn main() -> Result<()> {
    let document = json!({
        "scope": "UTXO-backed V4 fixture profile with P2PK signatures and a mandatory experimental vault guard; no node activation or native zUSD value pool",
        "units": {"zec": "zatoshi", "zusd": "micro-zUSD"},
        "scenarios": [scenario(None)?, scenario(Some(50))?, scenario(Some(20))?],
    });
    println!("{}", serde_json::to_string_pretty(&document)?);
    Ok(())
}
