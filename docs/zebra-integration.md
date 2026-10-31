# Native ZSV integration with Zebra

Implement the [candidate ZIP](../zips/draft-ijaack-zsv.md) as public transactions and state transitions. It specifies a dedicated ZEC vault value pool, public zUSD UTXOs, signatures, a 5-of-7 oracle and bad-debt retirement. No Zebra node currently enforces these rules.

The source baseline is Zebra `5faf8a413f0d30a6e93115f3d16af1e229fd7858` (28 October 2026), with ZIP research pinned at `2bdf39494320e5e04057ac93b1794031ebfb00b5`. See [sources.md](sources.md) for dependency versions and evidence. Recheck insertion points when updating the baseline.

## Use the existing transaction and state layers

At this pin, `zebra_chain::transaction::Transaction` wraps `zcash_primitives::transaction::Transaction`. Serialization delegates to the inner representation. Adding a Zebra-only accessor cannot introduce a native format: coordinated transaction-library changes are required.

The candidate needs trusted network context for hashing because it commits the network genesis hash as well as the branch ID. Its signatures bind actual resolved inputs and prior vault state. A JSON assertion or a model checkpoint cannot supply that authorization.

| Layer | Source location at the pin | Work |
|---|---|---|
| Transaction encoding and hashing | `zcash_primitives`; `zebra-chain/src/transaction.rs`, `transaction/compat.rs`, `transaction/hash.rs` | Add the candidate format, canonical parsing, effects/auth digests and witnessed relay dispatch. Use its closed ZEC/zUSD fields. |
| Block commitments | `zebra-chain/src/block/merkle.rs`, `block/commitment.rs` | Include the candidate txid and authDigest under the specified existing tree algorithms. |
| Semantic verification | `zebra-consensus/src/transaction.rs`, common checks and version dispatch | Enforce activation, structure, resource bounds and secp256k1/Ed25519 signatures. Preserve ordinary ZEC validation. |
| Contextual execution | `zebra-state/src/service/write.rs::validate_and_commit_non_finalized`; `service/non_finalized_state.rs::commit_block` | Resolve the exact parent and execute ordered vault, stable UTXO, oracle, supply and bad-debt changes in a private overlay. Use the parent oracle for all current-block pricing; commit any new report for descendants. Conserve ZEC across ordinary outputs, vault collateral and fees. |
| Forks and rollback | `service/non_finalized_state/chain.rs`: `push`, `fork`, `pop_root`; verified-block/treestate types | Carry and reverse the same ZSV state on every branch. |
| Finalization and persistence | `service/finalized_state.rs::commit_finalized_direct`; `service/finalized_state/column_family.rs`, `zebra_db.rs` | Validate or establish equivalent state on checkpoint paths; commit atomically and reproduce it after restart. |
| Mempool and templates | Transaction verifier; `zebrad/src/components/mempool/storage.rs`, `storage/verified_set.rs` | Track dependencies and conflicts; revalidate on changes to the tip, oracle or activation. Block validation remains authoritative. |
| Transport and costs | `zebra-network/src/protocol/external/codec.rs`; size and fee-policy paths | Bound parsing and verification work; support the specified relay identity and measure costs. |
| Activation | `zebra-chain/src/parameters/network_upgrade.rs`, `parameters/constants.rs` | Add identifiers and parameters allocated through the upgrade process; use isolated fixtures while developing. |

Vault collateral is held in its own value pool, not an ordinary UTXO. Ordinary transactions therefore have no vault outpoint to spend. Their existing rules must continue to exclude access to the new pools; only a validated ZSV transition can move value into or out of them. This avoids introducing the prototype's marker-script guard as a second collateral mechanism.

Public v1 needs no new ZK circuit or shielded stablecoin proof verifier. Existing Zcash shielded validation remains intact. A later shielded-payment ZIP may reuse selected ZSA techniques after separate specification and review.

## Evidence from the existing experiments

- [Contextual adapter](../zebra/contextual_adapter.rs): prepares and commits signed account-model transitions against an exact model parent. It is an in-memory experiment.
- [Block runner](zebra-development.md): parses native fixture blocks, binds model payloads and replays branches. Its data commitment does not lock ZEC.
- [Collateral profile](collateral-profile.md): checks restricted V4 signatures and fixture UTXOs against a separate model ledger. Its OP_TRUE vault marker is protected only by the harness. Trusted funding, dual key mappings and extra intent signatures are experimental scaffolding, not candidate requirements.

Preserve these experiments and their regression cases as evidence. Native implementation follows the ZIP directly; it does not need another adapter combining their different signing domains and state formats. Reserve research remains outside the core implementation.

## Acceptance evidence

Before a value-bearing deployment, demonstrate:

1. Complete signed transaction vectors, invalid encodings and cross-network/branch replay rejection, checked independently.
2. Block tests for double spends, vault conflicts, price ordering, rounding, supply limits, exact payouts and fee conservation.
3. Atomic rejection, fork/reorg, checkpoint, finalization and restart equivalence for ordinary ZEC and all new state.
4. Actual patched-node acceptance/rejection on an isolated network, wallet construction and measured verification/storage costs.
5. Qualified consensus, cryptographic, privacy and economic review, plus approved activation and reporter arrangements.

Use the existing [ZIP fixture checks](../zips/draft-ijaack-zsv/README.md) for their limited scope. `bash zebra/verify-dev.sh` checks the offline experiments with nextest; it does not test a patched node. Oracle truth, liquidation liquidity and willingness to absorb bad debt remain economic dependencies even when validation is correct.
