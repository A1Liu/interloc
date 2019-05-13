use crate::alloc::*;
use core::alloc::Layout;
use core::cell::RefCell;
use core::sync::atomic::{fence, Ordering};
use lock_api::RawRwLock as RawRwLockTrait;
use parking_lot::RawRwLock;

/// Information about allocs by the allocator
#[derive(Clone, Default, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AllocInfo {
    // Taken directly from https://github.com/neoeinstein/stats_alloc, or stats_alloc
    // on crates.io - all credit to the original writer of this struct, the user
    // neoeinstein on GitHub.
    /// Number of calls to alloc
    pub alloc: usize,
    /// Number of calls to dealloc
    pub dealloc: usize,
    /// Number of calls to realloc
    pub realloc: usize,
    /// Total bytes allocated
    pub bytes_alloc: usize,
    /// Total bytes deallocated
    pub bytes_dealloc: usize,
}

impl AllocInfo {
    pub const fn new() -> Self {
        Self {
            alloc: 0,
            dealloc: 0,
            realloc: 0,
            bytes_alloc: 0,
            bytes_dealloc: 0,
        }
    }
    pub fn relative_to(&self, origin: &Self) -> Self {
        Self {
            alloc: self.alloc - origin.alloc,
            dealloc: self.dealloc - origin.dealloc,
            realloc: self.realloc - origin.realloc,
            bytes_alloc: self.bytes_alloc - origin.bytes_alloc,
            bytes_dealloc: self.bytes_dealloc - origin.bytes_dealloc,
        }
    }

    #[inline]
    pub fn after_call(&self, layout: Layout, action: AllocAction) -> Self {
        use AllocAction::*;
        let mut info = *self;
        let size = layout.size();
        match action {
            Alloc | AllocZeroed => {
                info.alloc += 1;
                info.bytes_alloc += size;
                info
            }
            Dealloc { ptr: _ } => {
                info.dealloc += 1;
                info.bytes_dealloc += size;
                info
            }
            Realloc { ptr: _, new_size } => {
                info.realloc += 1;
                info.bytes_alloc += new_size;
                info.bytes_dealloc += size;
                info
            }
            _ => info,
        }
    }
}

pub struct StatsMonitor {
    info: AllocInfo,
    lock: RawRwLock,
}

impl StatsMonitor {
    /// New instance of this monitor.
    pub const fn new() -> Self {
        Self {
            info: AllocInfo::new(),
            lock: RawRwLock::INIT,
        }
    }

    #[inline]
    pub fn info(&self) -> AllocInfo {
        self.lock.lock_shared();
        fence(Ordering::SeqCst);
        let info = self.info;
        fence(Ordering::SeqCst);
        self.lock.unlock_shared();
        info
    }

    #[inline]
    pub fn write_info(&self, new_info: AllocInfo) {
        let info = &self.info as *const AllocInfo as *mut AllocInfo;
        self.lock.lock_exclusive();
        fence(Ordering::SeqCst);
        unsafe { *info = new_info };
        fence(Ordering::SeqCst);
        self.lock.unlock_exclusive();
    }
}

impl AllocMonitor for StatsMonitor {
    fn monitor(&self, layout: Layout, action: AllocAction) {
        let info = self.info();
        self.write_info(info.after_call(layout, action));
    }
}

/// Thread-local statistics on memory usage.
pub struct ThreadMonitor;

impl ThreadMonitor {
    thread_local! {
    static THREAD_INFO: RefCell<AllocInfo> = RefCell::new(AllocInfo::new());
    }

    pub const fn new() -> Self {
        Self {}
    }

    /// Returns an `AllocInfo` struct with information only related
    /// to the current thread of execution.
    pub fn info(&self) -> AllocInfo {
        Self::THREAD_INFO.with(|i| *i.borrow())
    }

    /// Writes to the history of the current thread of execution only.
    pub fn write_info(&self, info: AllocInfo) {
        Self::THREAD_INFO.with(|i| *i.borrow_mut() = info);
    }
}

impl AllocMonitor for ThreadMonitor {
    fn monitor(&self, layout: Layout, action: AllocAction) {
        self.write_info(self.info().after_call(layout, action));
    }
}
