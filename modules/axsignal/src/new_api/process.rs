use core::{
    array,
    ops::{Index, IndexMut},
};
use crate::{PendingSignals, SignalAction, Signo};
use axsync::{Mutex, spin::SpinNoIrq};
use axtask::WaitQueue;

/// Signal actions for a process.
pub struct SignalActions(pub(crate) [SignalAction; 64]);
impl Default for SignalActions {
    fn default() -> Self {
        Self(array::from_fn(|_| SignalAction::default()))
    }
}
impl Index<Signo> for SignalActions {
    type Output = SignalAction;
    fn index(&self, signo: Signo) -> &SignalAction {
        &self.0[signo as usize - 1]
    }
}
impl IndexMut<Signo> for SignalActions {
    fn index_mut(&mut self, signo: Signo) -> &mut SignalAction {
        &mut self.0[signo as usize - 1]
    }
}

/// Process-level signal manager.
pub struct ProcessSignalManager {
    /// The process-level shared pending signals
    _pending: SpinNoIrq<PendingSignals>,
    /// The signal actions
    pub actions: Mutex<SignalActions>,
    /// The wait queue for signal.
    pub signal_wq: WaitQueue,
}

impl ProcessSignalManager {
    /// Creates a new process signal manager.
    pub fn new() -> Self {
        Self {
            _pending: SpinNoIrq::new(PendingSignals::new()),
            actions: Mutex::new(SignalActions::default()),
            signal_wq: WaitQueue::new(),
        }
    }
}