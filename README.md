# `interloc`
This crate defines an interface for creating allocator middleware,
i.e. code that runs when your allocator is run.

# Examples
```rust
use interloc::{AllocMonitor, AllocAction, InterAlloc, StatsMonitor, ThreadMonitor};
use std::alloc::System;
use core::alloc::Layout;

struct MyMonitor {
    pub global: StatsMonitor,
    pub local: ThreadMonitor
}

impl MyMonitor {

    // This needs to be const to be usable in static variable declarations.
    pub const fn new() -> Self {
        Self {
            global: StatsMonitor::new(),
            local: ThreadMonitor::new(),
        }
    }
}

impl AllocMonitor for MyMonitor {

    // The immutable `&self` reference signature is there because the global allocator
    // needs to be thread-safe.
    fn monitor(&self, layout: Layout, action: AllocAction) {
        // Monitors are inherently composable
        self.global.monitor(layout, action);
        self.local.monitor(layout, action);
    }
}

static MONITOR: MyMonitor = MyMonitor::new();

// This needs to be done at the project root, i.e. `lib.rs` or `main.rs`
#[global_allocator]
static GLOBAL: InterAlloc<System, MyMonitor> = InterAlloc {
    inner: System,
    monitor: &MONITOR,
};
```
