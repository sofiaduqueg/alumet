//! Foreign Function Interface for dynamically-loaded plugins.
//! 
//! To be usable by plugins in a reliable way, every exposed
//! function needs to be declared like this:
//! ```ignore
//! #[no_mangle]
//! pub extern "C" fn(...) -> ... {
//!     // ...
//! }
//! ```
//! and every exposed struct needs to be repr-C:
//! ```ignore
//! #[repr(C)]
//! pub struct ExposedStruct {
//!     // ...
//! }
//! ```

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use libc::c_void;

use crate::config::ConfigTable;
use crate::measurement::{MeasurementAccumulator, MeasurementBuffer};
use crate::pipeline::OutputContext;
use crate::plugin::AlumetStart;
use time::{Timestamp, TimeDuration};

// Submodules
pub mod config;
pub mod metrics;
pub mod pipeline;
pub mod plugin;
pub mod resources;
pub mod units;
pub mod string;
pub mod time;

// ====== Function types ======
pub type PluginInitFn = extern "C" fn(config: *const ConfigTable) -> *mut c_void;
pub type PluginStartFn = extern "C" fn(instance: *mut c_void, alumet: *mut AlumetStart);
pub type PluginStopFn = extern "C" fn(instance: *mut c_void);
pub type DropFn = unsafe extern "C" fn(instance: *mut c_void);
pub type NullableDropFn = Option<unsafe extern "C" fn(instance: *mut c_void)>;

pub type SourcePollFn = extern "C" fn(instance: *mut c_void, buffer: *mut MeasurementAccumulator, timestamp: Timestamp);
pub type TransformApplyFn = extern "C" fn(instance: *mut c_void, buffer: *mut MeasurementBuffer);
pub type OutputWriteFn = extern "C" fn(instance: *mut c_void, buffer: *const MeasurementBuffer, ctx: *const FfiOutputContext);

// ====== OutputContext ======

#[repr(C)]
pub struct FfiOutputContext {
    inner: *const OutputContext
}
