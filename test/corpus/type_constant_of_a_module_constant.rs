// A type's constant that reads its module's constant of the same name, as
// chrono's `impl TimeDelta { pub const MIN: Self = MIN; }` reads `const
// MIN: TimeDelta` (#513's regression): each is its own JS constant, the
// type's its object's, and neither reads itself.
pub struct TimeDelta {
    pub secs: i64,
    pub nanos: i32,
}

const MILLIS_PER_SEC: i64 = 1000;

pub(crate) const MIN: TimeDelta = TimeDelta {
    secs: -i64::MAX / MILLIS_PER_SEC - 1,
    nanos: 7,
};

pub(crate) const MAX: TimeDelta = TimeDelta {
    secs: i64::MAX / MILLIS_PER_SEC,
    nanos: 9,
};

impl TimeDelta {
    pub const MIN: Self = MIN;
    pub const MAX: Self = MAX;

    pub const fn min_value() -> TimeDelta {
        MIN
    }

    pub fn in_range(&self) -> bool {
        self.secs >= MIN.secs && self.secs <= MAX.secs
    }
}

fn main() {
    let min = TimeDelta::MIN;
    let max = TimeDelta::MAX;
    let lowest = TimeDelta::min_value();
    println!("{} {} {} {}", min.secs, min.nanos, max.secs, max.nanos);
    println!("{} {}", lowest.secs, lowest.in_range());
    let outside = TimeDelta {
        secs: max.secs + 1,
        nanos: 0,
    };
    println!("{}", outside.in_range());
}
