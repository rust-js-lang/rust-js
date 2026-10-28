// React DOM's `ReactDOMUpdatePriority`: a lane its shared internals hold,
// read back as an `EventPriority` by the reconciler's `From`.
use dep::EventPriority;

pub fn main() {
    let priority = EventPriority::from(8);
    let into: EventPriority = 32.into();
    println!("{} {}", dep::event_priority_to_lane(priority), dep::event_priority_to_lane(into));
    println!("{}", priority == dep::DISCRETE_EVENT_PRIORITY);
    println!("{:?}", EventPriority::from(2) == dep::DISCRETE_EVENT_PRIORITY);
}
