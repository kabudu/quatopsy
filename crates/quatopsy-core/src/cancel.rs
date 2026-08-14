//! Cooperative cancellation for ingest and analysis.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::ingest::IngestError;

#[derive(Clone, Copy)]
pub struct Cancel<'a> {
    pub deadline: Instant,
    pub flag: Option<&'a AtomicBool>,
}

impl<'a> Cancel<'a> {
    pub fn check(&self) -> Result<(), IngestError> {
        if self.timed_out() {
            return Err(IngestError::failed_timeout());
        }
        if self.is_cancelled() {
            return Err(IngestError::failed_cancelled());
        }
        Ok(())
    }

    pub fn timed_out(self) -> bool {
        Instant::now() > self.deadline
    }

    pub fn is_cancelled(self) -> bool {
        self.flag.is_some_and(|flag| flag.load(Ordering::Relaxed))
    }
}
