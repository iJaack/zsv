//! Proposed contextual adapter seam; not linked into Zebra or a native Zcash format.
//!
//! The checkpoint below is the model's SHA-256 checkpoint, not a ZIP 244 digest,
//! Zcash block hash, or protocol state root. A real adapter must also select the
//! state belonging to the candidate block's exact parent branch inside Zebra.
use zsv::{Block, Error, Network, Snapshot, State};

/// Context supplied by the caller that owns the candidate block's parent branch.
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub network: Network,
    /// The candidate height; must equal the model block and parent height plus one.
    pub height: u64,
    /// Exact model checkpoint observed for that parent before preparation.
    pub expected_parent_commitment: [u8; 32],
}

#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    UnsupportedNetwork,
    ParentChanged,
    UnexpectedHeight,
    Model(Error),
}

/// An immutable, one-use candidate created only by successful model validation.
///
/// Private fields prevent callers from substituting an unverified result. This
/// value intentionally implements neither `Clone` nor deserialization.
#[derive(Debug)]
pub struct PreparedTransition {
    parent_commitment: [u8; 32],
    parent_height: u64,
    candidate: State,
}

impl PreparedTransition {
    /// Return a detached copy for inspection without allowing candidate mutation.
    pub fn snapshot(&self) -> Snapshot {
        self.candidate.snapshot()
    }
}

/// Validate real model signatures and ordered operations without changing parent.
pub fn validate(
    context: Context,
    parent: &State,
    block: &Block,
) -> Result<PreparedTransition, AdapterError> {
    if context.network != Network::Regtest {
        return Err(AdapterError::UnsupportedNetwork);
    }
    let parent_commitment = parent.commitment();
    if parent_commitment != context.expected_parent_commitment {
        return Err(AdapterError::ParentChanged);
    }
    let parent_height = parent.snapshot().height;
    let next_height = parent_height
        .checked_add(1)
        .ok_or(AdapterError::UnexpectedHeight)?;
    if context.height != next_height || block.height != next_height {
        return Err(AdapterError::UnexpectedHeight);
    }
    let candidate = parent.validate_block(block).map_err(AdapterError::Model)?;
    Ok(PreparedTransition {
        parent_commitment,
        parent_height,
        candidate,
    })
}

/// Atomically install a candidate only while its exact parent remains current.
///
/// Consuming the preparation prevents reuse. The exclusive mutable borrow spans
/// the check and replacement, with no callback or asynchronous gap between them.
pub fn commit(prepared: PreparedTransition, parent: &mut State) -> Result<(), AdapterError> {
    if parent.snapshot().height != prepared.parent_height
        || parent.commitment() != prepared.parent_commitment
    {
        return Err(AdapterError::ParentChanged);
    }
    *parent = prepared.candidate;
    Ok(())
}
