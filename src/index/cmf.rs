// src/index/cmf.rs

use super::{IndexBuilder, IndexError};
use crate::types::{IndexTick, CmfInputs};
use std::time::SystemTime;

pub struct CmfIndexBuilder;

impl CmfIndexBuilder {
    pub fn new() -> Self {
        Self
    }
}

impl IndexBuilder<CmfInputs> for CmfIndexBuilder {
    fn build(&mut self, tick: CmfInputs) -> Result<IndexTick, IndexError> {
        // Placeholder: compute a trivial index value from CMF inputs
        let price = tick.last_price;
        let ts: SystemTime = tick.ts;

        Ok(IndexTick { ts, price })
    }
}
