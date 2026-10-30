# Draft: native Zcash Stablecoin Vaults

ZIP: **unassigned**

Status: local research draft, not submitted; 29 October 2026.

Category: Consensus / Wallet.

## Summary

Introduce exactly one consensus-defined dollar-targeting stable asset backed by native ZEC vaults. Provide no general asset-issuance capability. A complete specification still needs the native encoding, proof statements, activation and economic decisions listed below. This draft does not reserve transaction versions, asset IDs or branch IDs.

## Required consensus behavior

The permitted asset set is a closed enum containing ZEC and the stable asset. No transaction may introduce a user-selected asset ID, issuer, description, NFT series or finalization flag. Any future extension of the enum requires a separate consensus change. Nodes and proofs MUST reject all other tags/bases, including encodings that wallets hide or ignore.

Stable issuance MUST equal principal added through a valid vault transition in the same transaction. There MUST be no alternate issuer-key authorization path. Stable repayment MUST consume actual stable value; a claimed burn amount or memo is insufficient. Stable asset flows MUST NOT modify ZEC monetary supply or replace ZEC transaction fees.

Vault locking MUST consume real native ZEC. Locked value MUST NOT also be spendable as an ordinary transparent output or shielded note. Each vault has exactly one current state, represented by a uniquely identified unspent vault output. The executable uses owner-scoped local integer IDs to prevent another owner from squatting that namespace. This model scheme is not the proposed native identifier scheme. Native outpoints/domain-separated 256-bit identifiers must be specified with creation/spend uniqueness.

Each transition MUST commit its owner, operation, amounts, asset types, previous vault reference, stable value inputs/outputs, successor state, network/branch, transaction version and relevant oracle epoch. A liquidator intent MUST also bound the amount burned and collateral received. Current owner authorization cannot override the transition verifier.

## Protocol state

Proposed state includes the unspent vault set, accepted oracle observation and reporter-set version, live principal, outstanding stable supply, explicit bad debt, stable payment UTXOs or note/nullifier state, and activation version. Consensus state is selected by the candidate block's parent branch, never implicitly by the current best-chain tip.

The optional reserve adds cash, existing stable inventory, immutable quote/funding/lifecycle parameters, epoch counters, retained funding, NSM allocation accounting, report inclusion height and shutdown/auction state. Its Rust implementation uses model balances; native value paths and persistent storage remain unspecified.

Stable supply MUST equal live principal plus bad debt when fees are zero. Stable spendable value MUST equal supply under the chosen public/shielded representation. ZEC value is conserved independently. Fees and interest, if introduced, need a revised invariant that tracks their actual allocation; they cannot be added silently.

The model uses full-state scans and cloning for clarity. A native implementation requires incremental, reversible deltas with durable atomic writes, bounds and reproducible finalization/restart behavior. Snapshot hashes in the model are not a specified Zcash state commitment.

## Operations

- **Open/deposit:** authorize a collateral contribution and create/update a vault. Do not require an oracle to add collateral.
- **Borrow:** require owner authorization, a fresh canonical price, sufficient collateral, minimum debt and global ceiling. Atomically mint principal. Reject any new borrowing with bad debt in the v0 policy.
- **Repay:** debit and burn stable value; decrease principal exactly; allow repayment without a fresh oracle. The executable prototype requires owner authorization for repayment; third-party repayment would need a separate specification.
- **Withdraw/close:** require owner authorization. Debt-free release needs no oracle. Otherwise enforce collateral sufficiency after release. No alternate payment path may release locked value.
- **Transfer:** conserve an existing permitted asset; do not alter debt or authorize issuance.
- **Liquidate:** require fresh oracle and unhealthy state; do not require borrower authorization. Burn supplied stablecoins, release precisely computed collateral, return residual collateral, remove the vault and record any uncovered liability. Include signed epoch and slippage/amount bounds.
- **Reserve trade:** transfer existing ZEC and stablecoin value atomically under canonical bid/ask rules. Bind direction, amount, report sequence, epoch, expiry and slippage limits to the user's authorization. Enforce aggregate epoch allowances without creating stable supply.
- **Reserve closure:** after the fixed outage deadline or sunset, burn the smaller of held inventory and recorded bad debt once, reducing both supply and bad debt. Route cash to the specified NSM path and offer residual inventory through a paid auction. Active principal and collateral remain unchanged. Auction purchases require actual ZEC and signed expiry/input bounds.

Units, integer inequalities and liquidation rounding are in [design.md](design.md). Model parameters are test values and are not recommended mainnet settings.

## Oracle

Signed observations are submitted through native transactions. Consensus validates reporter membership, quorum, distinct signers, identical tuple, monotonic sequence, defined validity window and canonical ordering. A node MUST NOT fetch a web price or use local wall-clock time while validating a block.

The proposal needs an explicit observed-height/not-before policy in addition to expiry. It also needs delayed reporter rotation, correlated-source analysis, equivocation handling, emergency policy and economic corruption analysis. The model implements sequence and expiry with a 100-block maximum acceptance lifetime; reserve quotes additionally bound inclusion age. Its signed reports carry no market-observation timestamp. A fresh inclusion therefore does not prove fresh market data. Distinct authorized keys do not establish independent observations.

## Reserve funding

Any native funding allocation MUST derive from a separately approved subsidy schedule exactly once per block. It MUST preserve native ZEC coinbase and NSM accounting, including excess credits and post-closure allocations. Initial cash plus later retained credits is capped; trading proceeds do not restore funding allowance. There is no user-submitted funding action or authority to allocate hypothetical future reductions in ZCG funding.

The executable's immutable regtest schedule creates simulated ZEC and defaults to zero funding. Its NSM bucket records value without implementing native deferred-subsidy issuance. These fixtures define accounting tests, not a proposed allocation amount or adopted schedule.

## Wire and crypto work still required

The source baseline's v6 transaction format is already allocated to NU6.3/Ironwood. Do not reuse it or historical withdrawn ZIP 230. A dedicated format/bundle proposal must define byte-level encodings and txid/auth digests coordinated with librustzcash. ZIP 248 is relevant draft infrastructure, not an implemented ZSV parser.

A fixed-two-asset shielded profile could reuse selected ZSA transfer/burn techniques. It MUST bind asset constants, note provenance, per-asset balance and public mint/burn amounts. Arbitrary ZSA issuer and registration machinery is excluded. Fixed asset constants do not remove the need for a cryptographic audit or soundness analysis.

## Failure and resource semantics

Invalid blocks MUST leave vault, oracle, ZEC, stable, debt, reserve, funding and nonce/nullifier state unchanged. Side branches, finalization, reorgs and restarts MUST reproduce identical state. Transaction validation and mempool admission MUST NOT bypass contextual branch verification. Coinbase MUST NOT create vault debt or the stable asset.

Byte/count/compute limits MUST apply before untrusted allocation or cryptographic verification. Protocol fees, debt-free-vault storage, retained identifiers, account/note churn and signature/proof costs need a denial-of-service policy. The isolated model's count limits alone are insufficient for a public node.

## Economic and privacy requirements

Native validity does not establish a dollar peg or guarantee solvency. Before activation, specify holder redemption/peg incentives, liquidation liquidity, recapitalization, bad-debt allocation and terminal settlement. The borrowing freeze has no reset authority and can be griefed by tiny shortfalls. Reserve closure covers only existing bad debt within its actual token inventory; later losses and uncovered debt remain. Adding an emergency spending key is not an acceptable unstated remedy.

Public vaults expose financial positions. Shielded payments can still correlate at issuance/burn boundaries. Fully private vaults require a liquidation witness-availability design that works without borrower cooperation.

## Activation and acceptance

No activation height or public network identifier is specified. The model rejects Mainnet and Testnet. Before a native deployment: freeze the transition/wire/proof specification; produce independent serialization/digest vectors; implement Zebra contextual/fork/storage integration; run multi-node private-network tests and cost benchmarks; resolve economic blockers; obtain independent consensus/crypto/security review; and seek community adoption through the appropriate network-upgrade process.

See [zebra-integration.md](zebra-integration.md) and [sources.md](sources.md) for pinned primary sources. This draft does not imply upstream acknowledgment, a ZIP assignment or permission to activate a public network.
