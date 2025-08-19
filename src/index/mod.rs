// src/index/mod.rs

pub mod cfd;
pub mod cmf;
pub mod cfd_consensus;

pub use cfd_consensus::CfdConsensus; // <— add this re-export

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("not enough data")]
    NotEnoughData,
    #[error("stale input")]
    StaleInput,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("internal: {0}")]
    Internal(String),
}

// Generic builder trait many index builders can implement.
pub trait IndexBuilder<I> {
    fn build(&mut self, tick: I) -> Result<crate::types::IndexTick, IndexError>;
}
