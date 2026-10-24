//! React's `shared/ReactComponentStackFrame` as react-in-rust ports it, with
//! the modules of its package it uses, each copied as it is there.

#[path = "ConsolePatchingDev.rs"]
pub mod console_patching_dev;
#[path = "DefaultPrepareStackTrace.rs"]
pub mod default_prepare_stack_trace;
pub mod environment;
#[path = "ReactComponentStackFrame.rs"]
pub mod react_component_stack_frame;
#[path = "ReactOwnerStackFrames.rs"]
pub mod react_owner_stack_frames;
#[path = "ReactSharedInternals.rs"]
pub mod react_shared_internals;
