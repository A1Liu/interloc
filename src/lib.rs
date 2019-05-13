//! # `interloc`
//! This crate defines an interface for creating allocator middleware,
//! i.e. code that runs when your allocator is run.
//!
//! # Examples
//! ```rust
//! use interloc::{AllocMonitor, AllocAction, InterAlloc, StatsMonitor, ThreadMonitor};
//! use std::alloc::System;
//! use core::alloc::Layout;
//!
//! struct MyMonitor {
//!     pub global: StatsMonitor,
//!     pub local: ThreadMonitor
//! }
//!
//! impl MyMonitor {
//!
//!     // This needs to be const to be usable in static functions
//!     pub const fn new() -> Self {
//!         Self {
//!             global: StatsMonitor::new(),
//!             local: ThreadMonitor::new(),
//!         }
//!     }
//! }
//!
//! impl AllocMonitor for MyMonitor {
//!     fn monitor(&self, layout: Layout, action: AllocAction) {
//!         self.global.monitor(layout, action);
//!         self.local.monitor(layout, action);
//!     }
//! }
//!
//! static MONITOR: MyMonitor = MyMonitor::new();
//!
//! // This needs to be done at the project root
//! #[global_allocator]
//! static GLOBAL: InterAlloc<System, MyMonitor> = InterAlloc {
//!     inner: System,
//!     monitor: &MONITOR,
//! };
//! ```
extern crate lock_api;
extern crate parking_lot;

mod alloc;
mod monitor;

pub use alloc::*;
pub use monitor::*;
