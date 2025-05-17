//! Futex table for thread synchronization.

use core::ops::Deref;

use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use axsync::Mutex;
use axtask::{TaskExtRef, WaitQueue, current};

/// Maps futex addresses to their wait queues for efficient thread synchronization.
pub struct FutexTable(Mutex<BTreeMap<usize, Arc<WaitQueue>>>);

impl FutexTable {
    /// Creates a new empty futex table.
    pub fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    /// Gets an existing wait queue for the given address, or returns None if not found.
    pub fn get(&self, addr: usize) -> Option<WaitQueueGuard> {
        let wq = self.0.lock().get(&addr).cloned()?;
        Some(WaitQueueGuard {
            key: addr,
            inner: wq,
        })
    }

    /// Gets or creates a wait queue for the given address.
    pub fn get_or_insert(&self, addr: usize) -> WaitQueueGuard {
        let mut table = self.0.lock();
        let wq = table
            .entry(addr)
            .or_insert_with(|| Arc::new(WaitQueue::new()));
        WaitQueueGuard {
            key: addr,
            inner: wq.clone(),
        }
    }
}

/// Smart pointer wrapper for wait queues that provides automatic cleanup on drop.
pub struct WaitQueueGuard {
    /// The memory address associated with this wait queue
    key: usize,
    /// The actual wait queue, wrapped in an Arc for shared ownership
    inner: Arc<WaitQueue>,
}

/// Allows WaitQueueGuard to be used like an Arc<WaitQueue> reference.
impl Deref for WaitQueueGuard {
    type Target = Arc<WaitQueue>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Removes the wait queue from the table when it's no longer needed.
impl Drop for WaitQueueGuard {
    fn drop(&mut self) {
        let curr = current();
        let mut table = curr.task_ext().process_data().futex_table.0.lock();
        if Arc::strong_count(&self.inner) == 1 && self.inner.is_empty() {
            table.remove(&self.key);
        }
    }
}
