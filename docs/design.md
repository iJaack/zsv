# Zcash Stablecoin Vault: scope and design

Status: experimental design, 29 October 2026. The user authorized the prototype and its supporting documents in chat. This document records implementation choices, not community acceptance.

The [ZIP candidate](../zips/draft-ijaack-zsv.md) is the primary proposed consensus specification. This document describes research-model choices; the candidate's native wire format, oracle and bad-debt retirement extend beyond that implementation.

## Goal

Specify a ZEC-collateralized, dollar-targeting stablecoin with all spending, issuance, debt and liquidation rules enforced by Zcash consensus. Provide an executable, adversarially tested reference model and a source-pinned implementation proposal for Zebra.

This is an alternative to activating general-purpose Zcash Shielded Assets. It may reuse reviewed components of their multi-asset accounting and shielded note machinery, but must expose only ZEC and one protocol-defined stablecoin, provisionally called zUSD. The name is descriptive and is not a ticker ownership claim.

## Approaches considered

1. **Dedicated native vault bundle (selected).** Fixed transitions and fixed asset types, explicit contextual state checks in Zebra, and narrowly scoped wallet/transaction support in librustzcash. This limits native issuance and makes supply/debt invariants directly auditable. It requires an adopted consensus upgrade and introduces financial/oracle complexity into the node.
2. **Generic proof/predicate extension.** A Zcash verifier could enforce application proofs. More flexible and potentially private but expands the application surface, verifier costs and design burden. Generic deployment permissions could recreate the issuance surface the user wants to avoid. Not selected for v0.
3. **External chain or threshold custody.** Technically feasible but fails the user's requirement that rules stay on Zcash. Excluded.

## Executable v0

The reference executable is an isolated Rust account-ledger model, not a Zebra transaction parser. It demonstrates authenticated state transitions and asset conservation without touching network funds. It does not implement shielded transfers, consensus activation, PoW, ZIP 244 transaction hashes, wallet recovery, fees, interest, redemption or general recapitalization.

One ZEC is 100,000,000 zatoshi. One zUSD is 1,000,000 micro-units. All calculations use checked integers. Opening/borrowing and withdrawals against debt require 250% collateralization. Liquidation is allowed strictly below 200%. Equality is healthy. Bonus is 5%. Minimum nonzero vault debt is 100 zUSD; global outstanding principal plus bad debt is capped at 100,000 zUSD. These are illustrative test parameters, not calibrated risk settings.

Signed owner requests bind the exact action, account nonce and immutable model-chain domain derived from the canonical configuration and genesis allocations. Vault identifiers are scoped by the owner public key and a local integer ID; liquidation identifies that owner explicitly. Identical canonical genesis allocations and configurations intentionally identify the same model chain, including its forks. At least two of three configured reporters independently sign each oracle report. Observations have a monotonic sequence and a height validity window. The currently accepted report is canonical. Reporter independence, price quality, quorum governance and economic corruption resistance remain external assumptions.

Every accepted transition is atomic. Invalid transactions/blocks leave state unchanged. Supply equals live vault principal plus explicit bad debt; wallet balances plus reserve inventory equal supply. Without the reserve, fixture ZEC is conserved between free balances and vault collateral. In reserve mode, the immutable fixture funding schedule adds simulated ZEC once per block. Wallets, vault collateral, reserve cash and the NSM bucket together equal genesis ZEC plus cumulative scheduled funding. There is no user-callable funding or unrestricted mint action.

## Liquidation

For collateral C in zatoshi, debt D in micro-zUSD and price P in micro-USD/ZEC, define:

    x = min(D, floor(C * P * 100 / (100000000 * 105)))
    z = ceil(x * 100000000 * 105 / (P * 100))

The signed liquidation action commits the canonical oracle sequence, maximum burn and minimum collateral received; all three bounds are checked before mutation. The liquidator burns x stable units, receives z zatoshi, and the owner receives C-z. The vault closes; D-x is recorded as bad debt. Checked arithmetic must establish z<=C rather than silently clamp an overflow. Liquidation does not require the borrower's cooperation. A price so low that x=0 permits a dust close but creates no economic incentive to submit it. The ledger does not promise liquidation liveness.

Any bad debt disables new borrowing. Existing repayment, collateral additions, transfers and permitted closure remain possible. Reserve closure can burn its own inventory against recorded bad debt. Any remaining shortfall stays explicit and keeps borrowing frozen; cash-funded recapitalization remains unimplemented.

## Optional reserve ledger

The reserve-enabled constructor binds its immutable policy, named donor and initial ZEC amount to a new model-chain domain. It debits that donor's genesis balance and starts with zero stable inventory. The original constructor and signed scenario vectors retain their previous behavior.

The reserve buys existing zUSD at a fixed dollar bid and sells existing inventory at a fixed dollar ask, using the accepted ZEC/USD report for conversion. Signed requests bind the quantity, direction, account nonce, canonical report sequence, derived epoch, expiry and minimum-output or maximum-input bound. Per-epoch gross allowances apply cumulatively across all transactions in a block. Sales retain their ZEC proceeds without restoring funding headroom.

Funding is derived from an immutable regtest schedule that defaults to zero per block. A credit is retained only within both the cash target and the lifetime retained-funding budget; excess goes to the NSM accounting bucket. Initial cash counts toward the budget. Before reserve activation and after closure, every credit goes to that bucket. The bucket is a model balance, not an implementation of native NSM subsidy rules or an approved Zcash allocation.

Normal quotes pause when report inclusion age or expiry exceeds the reserve policy. Reports have no signed market-observation timestamp, so these checks cannot detect a quorum publishing old market data as a new report. Vault valuation retains its existing report-expiry check. An elapsed outage deadline or sunset closes normal trading before a new report can rescue it. Closure burns the smaller of reserve inventory and bad debt once, reduces supply and bad debt equally, and moves all reserve cash to the NSM bucket. That burn precedes user transactions, so later liquidation losses remain uncovered. Active principal and collateral remain unchanged. Residual tokens enter a descending-price ZEC auction with signed expiry and maximum-input bounds; it needs no conversion oracle. Auction proceeds and subsequent funding go to the NSM bucket.

All reserve, funding and lifecycle transitions share the vault ledger's block overlay and conservation checks. The [Rust integration specification](reserve-rust-contract.md) records the exact API and ordering. The [lifecycle study](reserve-lifecycle.md) explains the economic limits: quota occupation, weak auction competition and uncovered bad debt remain possible.

## Zebra-linked development

The [offline harness](zebra-development.md) imports the pinned Zebra library to parse native blocks, verify coinbase data commitments and select exact parent model states by native block hash. It replays accepted envelopes using a separately trusted journal digest and tip. Its fixtures carry no native spendable funds; no node service, native vault lock or stablecoin value pool is implemented by this data-binding experiment.

## Native proposal

Public vault state and public liquidation information are the initial integration target. A proposed dedicated vault bundle binds collateral locking, successor vault state and zUSD mint/burn in one Zcash transaction. Contextual verification and branch-specific state commits/rollback must be atomic. Existing transparent or shielded payment paths must not provide alternative collateral spending or stablecoin issuance paths.

Shielded stablecoin payments could use a fixed-two-asset circuit profile in a later phase. ZSA issuer APIs, arbitrary AssetIds, custom descriptions, finalization controls and asset registries are excluded. Reusing selected ZSA components creates a new protocol requiring its own audit.

## Security and anti-memecoin boundary

The extension provides no consensus-recognized arbitrary native token issuance. It does not prevent inscriptions, external contracts, off-chain claims, wrapped derivatives, speculation in ZEC/zUSD or a future community-approved upgrade. Asset-ID restrictions alone cannot guarantee that the token always trades at one dollar.

## Deliverables and acceptance

- Rust model with real transaction/oracle signatures, checked monetary math, atomic state transitions and executable scenarios.
- Whitepaper and forum draft accurately separating implemented behavior and proposals.
- Source-based threat model and independent economic/code adversarial reviews, with residual blockers retained.
- Source-pinned Zebra/librustzcash integration plan, and an illustrative code seam if useful, explicitly unactivated.
- Reproducible test/build commands and verification receipt.
- A standalone GitHub repository named zsv under the authenticated user's account, without private workspace context or credentials.

No upstream submission, forum publication, mainnet activation, external custody, trading, background service or real financial action is authorized by this implementation task.
