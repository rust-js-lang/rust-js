use std::rc::Rc;

pub mod priorities {
    pub type PriorityLevel = u8;

    pub const NORMAL_PRIORITY: PriorityLevel = 3;
}

use priorities::PriorityLevel;

pub struct Task {
    pub id: u32,
    pub priority_level: PriorityLevel,
}

/// What `scheduleCallback` is: the host's, or this crate's.
type ScheduleCallback = dyn Fn(PriorityLevel, &str, Option<f64>) -> Rc<Task>;

pub fn schedule_callback(priority_level: PriorityLevel, label: &str, delay: Option<f64>) -> Rc<Task> {
    let delay = delay.unwrap_or(0.0) as u32;
    Rc::new(Task {
        id: label.len() as u32 + delay,
        priority_level,
    })
}

pub fn current_priority_level() -> PriorityLevel {
    priorities::NORMAL_PRIORITY
}

unsafe extern "Rust" {
    /// A host's scheduler, where one installs it.
    #[link_name = "hostScheduler"]
    pub safe static HOST_SCHEDULER: &'static js::Unknown;
}

thread_local! {
    pub static SCHEDULE_CALLBACK: &'static ScheduleCallback = if js::is_defined(HOST_SCHEDULER) {
        // SAFETY: the host's is a scheduler's, as this crate's is.
        unsafe { js::cast(js::get(HOST_SCHEDULER, "scheduleCallback")) }
    } else {
        &schedule_callback
    };

    pub static GET_CURRENT_PRIORITY_LEVEL: &'static dyn Fn() -> PriorityLevel = &current_priority_level;
}

fn scaled(value: f64, factor: f64) -> f64 {
    value * factor
}

pub static SCALE: fn(f64, f64) -> f64 = scaled;

/// What a visit asks of each value.
pub type Visit<'a, T> = dyn Fn(&T) -> bool + 'a;

pub fn visit<T>(value: &T, f: &Visit<'_, T>) -> bool {
    f(value)
}
