use crate::{PendingSignals, SignalSet, SignalStack};
use axsync::{Mutex, spin::SpinNoIrq};

/// Thread-level signal manager.
pub struct ThreadSignalManager {
    /// The pending signals
    pending: SpinNoIrq<PendingSignals>,
    /// The set of signals currently blocked from delivery
    blocked: Mutex<SignalSet>,
    /// The stack used by signal handlers
    stack: Mutex<SignalStack>,
}

impl ThreadSignalManager {
    pub fn new() -> Self {
        Self {
            pending: SpinNoIrq::new(PendingSignals::new()),
            blocked: Mutex::new(SignalSet::default()),
            stack: Mutex::new(SignalStack::default()),
        }
    }
}