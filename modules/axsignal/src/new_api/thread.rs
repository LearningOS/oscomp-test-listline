use super::ProcessSignalManager;
use crate::{PendingSignals, SignalSet, SignalStack};
use axsync::{Mutex, spin::SpinNoIrq};
use alloc::sync::Arc;
/// Thread-level signal manager.
pub struct ThreadSignalManager {
    /// The pending signals
    _pending: SpinNoIrq<PendingSignals>,
    /// The set of signals currently blocked from delivery
    _blocked: Mutex<SignalSet>,
    /// The stack used by signal handlers
    _stack: Mutex<SignalStack>,
}

impl ThreadSignalManager {
    pub fn new() -> Self {
        Self {
            _pending: SpinNoIrq::new(PendingSignals::new()),
            _blocked: Mutex::new(SignalSet::default()),
            _stack: Mutex::new(SignalStack::default()),
        }
    }
}
