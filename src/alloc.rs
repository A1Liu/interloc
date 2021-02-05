use core::alloc::GlobalAlloc;
pub use core::alloc::Layout;
use core::sync::atomic::{fence, Ordering};

/// An action that an allocator can take, either right before, or right after it
/// happens.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AllocAction {
    /// alloc was called
    Alloc,
    /// alloc returned a pointer
    AllocResult { ptr: *mut u8 },
    /// alloc_zeroed was called
    AllocZeroed,
    /// alloc_zeroed returned a pointer
    AllocZeroedResult { ptr: *mut u8 },
    /// dealloc was called on a pointer
    Dealloc { ptr: *mut u8 },
    /// dealloc finished execution
    DeallocResult,
    /// realloc was called on a pointer
    Realloc { ptr: *mut u8, new_size: usize },
    /// realloc returned a pointer
    ReallocResult { ptr: *mut u8, new_size: usize },
}

/// Before or after an allocation call is executed.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AllocRel {
    Before,
    After,
}

impl AllocAction {
    /// Whether the action is before or after the action itself.
    #[inline]
    pub fn relation(&self) -> AllocRel {
        use AllocAction::*;
        match self {
            Alloc | AllocZeroed => AllocRel::Before,
            Dealloc { ptr: _ } => AllocRel::Before,
            Realloc {
                ptr: _,
                new_size: _,
            } => AllocRel::Before,
            _ => AllocRel::After,
        }
    }
}

/// An allocator that watches the calls to its API, sends them to a struct, and
/// then makes the calls.
///
/// To use this struct, use the `#[global_allocator]` compiler directive and
/// construct the allocator with your own custom monitor, or one of the builtins.
/// Note that the new method of `interloc::StatsMonitor` is a `const fn`.
pub struct InterAlloc<'a, T, F>
where
    T: GlobalAlloc,
    F: AllocMonitor,
{
    // Taken directly from https://github.com/neoeinstein/stats_alloc, or stats_alloc
    // on crates.io - all credit to the original writer of this struct, the user
    // neoeinstein on GitHub.
    /// Inner allocator
    pub inner: T,
    /// Monitor on the calls to the allocator
    pub monitor: &'a F,
}

impl<'a, T, F> InterAlloc<'a, T, F>
where
    T: GlobalAlloc,
    F: AllocMonitor,
{
    /// A new instance of the allocator.
    #[inline]
    pub fn new(base: T, monitor: &'a F) -> Self {
        Self {
            inner: base,
            monitor: monitor,
        }
    }

    /// Call the monitor function.
    #[inline]
    fn monitor_(&self, layout: Layout, act: AllocAction) {
        self.monitor.monitor(layout, act);
    }
}

unsafe impl<'a, T, F> GlobalAlloc for InterAlloc<'a, T, F>
where
    T: GlobalAlloc,
    F: AllocMonitor,
{
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.monitor_(layout, AllocAction::Alloc);
        fence(Ordering::SeqCst);
        let ptr = self.inner.alloc(layout);
        fence(Ordering::SeqCst);
        self.monitor_(layout, AllocAction::AllocResult { ptr });
        ptr
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.monitor_(layout, AllocAction::Dealloc { ptr });
        fence(Ordering::SeqCst);
        self.inner.dealloc(ptr, layout);
        fence(Ordering::SeqCst);
        self.monitor_(layout, AllocAction::DeallocResult);
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.monitor_(layout, AllocAction::AllocZeroed);
        fence(Ordering::SeqCst);
        let ptr = self.inner.alloc_zeroed(layout);
        fence(Ordering::SeqCst);
        self.monitor_(layout, AllocAction::AllocZeroedResult { ptr });
        ptr
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.monitor_(layout, AllocAction::Realloc { ptr, new_size });
        fence(Ordering::SeqCst);
        let ptr = self.inner.realloc(ptr, layout, new_size);
        fence(Ordering::SeqCst);
        self.monitor_(layout, AllocAction::ReallocResult { ptr, new_size });
        ptr
    }
}

/// When attached to an `InterAlloc` instance, this struct's `monitor` method
/// is called before and after calls to the inner allocator. The ordering of
/// these method calls is enforced by `std::sync::atomic::fence`
/// with `std::sync::atomic::Ordering::SeqCst`.
pub trait AllocMonitor {
    /// The api to the monitor. This method is called right before and right after
    /// allocations happen.
    fn monitor(&self, layout: Layout, act: AllocAction);
}
