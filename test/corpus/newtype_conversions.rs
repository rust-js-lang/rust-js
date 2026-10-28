// A `From` between a `#[repr(transparent)]` struct and its field, as
// React's `EventPriority` of a `Lane`: the struct is its field in JS, so
// the conversion is the value it's given, `getCurrentUpdatePriority()`'s
// `ReactDOMSharedInternals.p`. One that does more, or of a struct laid out
// as itself, is called.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
struct EventPriority(i32);

impl From<i32> for EventPriority {
    fn from(lane: i32) -> Self {
        EventPriority(lane)
    }
}

impl From<EventPriority> for i32 {
    fn from(priority: EventPriority) -> Self {
        priority.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
struct Doubled(i32);

impl From<i32> for Doubled {
    fn from(lane: i32) -> Self {
        Doubled(lane * 2)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Boxed(i32);

impl From<i32> for Boxed {
    fn from(lane: i32) -> Self {
        Boxed(lane)
    }
}

fn current(lane: i32) -> EventPriority {
    EventPriority::from(lane)
}

fn back(priority: EventPriority) -> i32 {
    i32::from(priority)
}

fn into(lane: i32) -> EventPriority {
    lane.into()
}

fn maybe(lane: Option<i32>) -> Option<EventPriority> {
    lane.map(EventPriority::from)
}

fn main() {
    let lanes = [0, 1, 8, -2147483648];
    for lane in lanes {
        let priority = current(lane);
        println!("{priority:?} {} {:?}", back(priority), into(lane));
        println!("{:?} {:?}", Doubled::from(lane), Boxed::from(lane));
    }
    println!("{:?} {:?}", maybe(Some(4)), maybe(None));
}
