//! `nativeRuntimeScheduler`: React Native's scheduler, which its C++ runtime
//! installs as a global (`RuntimeSchedulerBinding.cpp`), and React's
//! `scheduler/src/forks/SchedulerNative.js` reads in its place. Each member
//! as Flow's `NativeSchedulerType` declares it, of the host's own types: its
//! priorities the scheduler's `PriorityLevel`s, whole numbers 1 to 5; a
//! task the host's own object, which only `unstable_cancelCallback` takes
//! (ADR 1083).

use core::marker::PhantomData;

use js::{Function, JsObject};

unsafe extern "Rust" {
    /// The `nativeRuntimeScheduler` global, which React Native's host
    /// installs, and no other host has: `js::is_defined(native_runtime_scheduler)`
    /// is `typeof nativeRuntimeScheduler !== "undefined"`, and read where
    /// it isn't, JS's `ReferenceError`, a panic.
    #[link_name = "nativeRuntimeScheduler"]
    #[allow(non_upper_case_globals)]
    pub safe static native_runtime_scheduler: &'static NativeScheduler;
}

/// What `nativeRuntimeScheduler` is: Flow's `NativeSchedulerType`, the
/// scheduler React Native's C++ runtime implements.
pub struct NativeScheduler(PhantomData<JsObject>);

unsafe impl js::Defined for NativeScheduler {}

/// A task the host scheduled: its own object, which
/// [`NativeScheduler::unstable_cancel_callback`] cancels, and nothing else
/// reads.
#[cfg_attr(rust_js, rust_js::types = "object")]
pub struct Task(PhantomData<JsObject>);

unsafe impl js::Defined for Task {}

/// A task's work, as the host calls it: given whether it timed out, it does
/// some, and gives what's left of it, a function the host calls next, or
/// nothing where it's done, Flow's `(didTimeout: boolean) => ?Callback`.
#[repr(transparent)]
pub struct Callback(pub Box<dyn FnMut(bool) -> Option<&'static Function>>);

impl NativeScheduler {
    /// `unstable_ImmediatePriority`: the priority of what runs at once, `1`.
    #[cfg_attr(rust_js, rust_js::link_name = "get unstable_ImmediatePriority")]
    pub fn unstable_immediate_priority(&self) -> u8 {
        unreachable!()
    }

    /// `unstable_UserBlockingPriority`: what a person waits on, `2`.
    #[cfg_attr(rust_js, rust_js::link_name = "get unstable_UserBlockingPriority")]
    pub fn unstable_user_blocking_priority(&self) -> u8 {
        unreachable!()
    }

    /// `unstable_NormalPriority`: the default, `3`.
    #[cfg_attr(rust_js, rust_js::link_name = "get unstable_NormalPriority")]
    pub fn unstable_normal_priority(&self) -> u8 {
        unreachable!()
    }

    /// `unstable_LowPriority`: what may wait, `4`.
    #[cfg_attr(rust_js, rust_js::link_name = "get unstable_LowPriority")]
    pub fn unstable_low_priority(&self) -> u8 {
        unreachable!()
    }

    /// `unstable_IdlePriority`: what runs when nothing else does, `5`.
    #[cfg_attr(rust_js, rust_js::link_name = "get unstable_IdlePriority")]
    pub fn unstable_idle_priority(&self) -> u8 {
        unreachable!()
    }

    /// `unstable_scheduleCallback(priorityLevel, callback)`: `callback`
    /// run at that priority, and the task that runs it.
    #[cfg_attr(rust_js, rust_js::link_name = "unstable_scheduleCallback")]
    pub fn unstable_schedule_callback(&self, priority_level: u8, callback: Callback) -> &'static Task {
        unreachable!()
    }

    /// `unstable_cancelCallback(task)`: the task not run, where it hasn't.
    #[cfg_attr(rust_js, rust_js::link_name = "unstable_cancelCallback")]
    pub fn unstable_cancel_callback(&self, task: &Task) {
        unreachable!()
    }

    /// `unstable_getCurrentPriorityLevel()`: the priority of what runs now.
    #[cfg_attr(rust_js, rust_js::link_name = "unstable_getCurrentPriorityLevel")]
    pub fn unstable_get_current_priority_level(&self) -> u8 {
        unreachable!()
    }

    /// `unstable_shouldYield()`: whether what runs should give the host its
    /// turn.
    #[cfg_attr(rust_js, rust_js::link_name = "unstable_shouldYield")]
    pub fn unstable_should_yield(&self) -> bool {
        unreachable!()
    }

    /// `unstable_requestPaint()`: a paint asked for at the next yield.
    #[cfg_attr(rust_js, rust_js::link_name = "unstable_requestPaint")]
    pub fn unstable_request_paint(&self) {
        unreachable!()
    }

    /// `unstable_now()`: the time now, in milliseconds, a
    /// `DOMHighResTimeStamp`.
    #[cfg_attr(rust_js, rust_js::link_name = "unstable_now")]
    pub fn unstable_now(&self) -> f64 {
        unreachable!()
    }
}
