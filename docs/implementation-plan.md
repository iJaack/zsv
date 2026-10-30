# ZSV implementation plan

Implement the [candidate ZIP](../zips/draft-ijaack-zsv.md): public ZEC vaults, one public stablecoin, an authenticated price feed and explicit bad debt. The [Zebra source map](zebra-integration.md) identifies the integration points. The ZIP is Draft and has not been submitted upstream.

## Scope

The initial implementation needs no new shielded circuit, ZSA issuance registry, reserve, reward controller, funding allocation or auction. It uses the candidate's dedicated collateral pool rather than promoting the experimental OP_TRUE vault marker into consensus.

Keep the account model and offline harnesses as research evidence. Do not add another wrapper around their JSON requests, model/native key mapping or trusted funding bootstrap to implement the new format. Reuse checked arithmetic and adversarial cases where they match the candidate; implement native authorization and state access directly.

## Next steps

1. **Review the core choices.** Use the forum draft to seek feedback on public payments, oracle trust, liquidation and the indefinite borrowing freeze. Confirm transaction-library feasibility and economic assumptions before expanding implementation. Public posting and upstream submission still need authorization.
2. **Implement native transactions and transitions.** Add the bounded encoding and digests to the transaction library used by Zebra, signature verification, public stable outputs and vault-pool accounting. Cover each operation with complete signed valid/invalid vectors. The current unsigned fixtures are only a starting point.
3. **Integrate branch state.** Validate from the exact parent, apply transactions in order and commit ordinary ZEC and ZSV state atomically. Cover mempool revalidation, forks, finalization, checkpoints and restart. No acceptance path may omit the new rules.
4. **Validate an isolated node implementation.** Compare independent results, measure resource costs and test wallet construction and adversarial blocks. Obtain qualified consensus, cryptographic, privacy and economic review before deployment. Network identifiers, activation and actual reporter keys require the upgrade process.

## Evidence already available

- Financial, unsigned-wire and canonical-oracle fixtures: [conformance material](../zips/draft-ijaack-zsv/README.md).
- Account-model and economic results: [verification receipt](verification.md).
- Native fixture ownership and collateral reconciliation: [collateral review](collateral-review.md).
- Submission requirements and remaining review: [ZIP readiness](zip-readiness.md).

These results cover their stated experiments. They do not establish native ZIP conformance or a stable dollar price. Shielding and stabilization funding can be evaluated separately after review of the core design.
