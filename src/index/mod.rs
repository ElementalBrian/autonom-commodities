// src/index/mod.rs
use crate::types::{IndexTick, CmfInputs, CfdTick};
pub mod cmf;
pub mod cfd;

#[derive(thiserror::Error, Debug)]
pub enum IndexError {
    #[error("stale input")]
    Stale,
    #[error("jump rejection")]
    Jump,
    #[error("hours closed")]
    Hours,
    #[error("no data")]
    NoData,
    #[error("internal: {0}")]
    Internal(&'static str),
}

pub trait IndexBuilder<I> {
    fn build(&mut self, inp: I) -> Result<IndexTick, IndexError>;
}

pub mod cfd_consensus;

