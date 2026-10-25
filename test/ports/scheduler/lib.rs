//! React's `scheduler` package, its DOM fork, as react-in-rust ports it:
//! `forks/Scheduler.rs`, its package's root there, with the modules it
//! declares, each copied as it is there; and `shared`'s `environment`,
//! which `SchedulerFeatureFlags.rs` reads as `shared::environment`, this
//! crate's own here.

extern crate self as shared;

pub mod environment;
#[path = "forks/Scheduler.rs"]
pub mod scheduler;
