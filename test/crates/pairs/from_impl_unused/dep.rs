// React's reconciler's `ReactEventPriorities`: an `EventPriority` is a
// lane, and a lane read back is one by `From`, which nothing of this crate
// calls, so its JS has no function of it (ADR 0477, amended).
pub type Lane = i32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct EventPriority(Lane);

impl From<Lane> for EventPriority {
    fn from(lane: Lane) -> Self {
        EventPriority(lane)
    }
}

pub const DISCRETE_EVENT_PRIORITY: EventPriority = EventPriority(2);

pub fn event_priority_to_lane(priority: EventPriority) -> Lane {
    priority.0
}
