// `contains` of a range of a crate's own `PartialOrd` type compares through
// its `PartialOrd`, as std's `RangeBounds::contains` does: the start's test,
// then the end's, which runs only if the start's holds (#1158).
use std::cmp::Ordering;
use std::ops::RangeInclusive;

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug)]
struct Day(u32);

#[derive(PartialEq, PartialOrd, Debug)]
struct Date {
    year: i32,
    day: Day,
}

// A hand-written order that says which comparisons it makes.
#[derive(PartialEq, Debug)]
struct Loud(f64);

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        println!("  compare {} with {}", self.0, other.0);
        self.0.partial_cmp(&other.0)
    }
}

// Of another type than the range's, as `PartialOrd<U>` allows.
#[derive(PartialEq, PartialOrd, Debug)]
struct Meters(f64);

impl PartialEq<f64> for Meters {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f64> for Meters {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<Meters> for f64 {
    fn eq(&self, other: &Meters) -> bool {
        *self == other.0
    }
}

impl PartialOrd<Meters> for f64 {
    fn partial_cmp(&self, other: &Meters) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

fn week(first: u32) -> RangeInclusive<Day> {
    Day(first)..=Day(first + 6)
}

fn main() {
    for d in [0, 1, 3, 5, 6] {
        println!(
            "{d}: {} {} {} {} {}",
            (Day(1)..=Day(5)).contains(&Day(d)),
            (Day(1)..Day(5)).contains(&Day(d)),
            (Day(1)..).contains(&Day(d)),
            (..Day(5)).contains(&Day(d)),
            (..=Day(5)).contains(&Day(d)),
        );
    }
    println!("{}", week(10).contains(&Day(16)));
    let r = week(10);
    println!("{} {:?}", r.contains(&Day(17)), r);
    let dates = Date { year: 2024, day: Day(3) }..Date { year: 2025, day: Day(1) };
    println!(
        "{} {} {}",
        dates.contains(&Date { year: 2024, day: Day(9) }),
        dates.contains(&Date { year: 2025, day: Day(1) }),
        dates.contains(&Date { year: 2023, day: Day(9) }),
    );
    println!("loud, below:");
    println!("{}", (Loud(1.0)..=Loud(3.0)).contains(&Loud(0.5)));
    println!("loud, inside:");
    println!("{}", (Loud(1.0)..Loud(3.0)).contains(&Loud(2.0)));
    println!("loud, NaN:");
    println!("{}", (Loud(1.0)..Loud(3.0)).contains(&Loud(f64::NAN)));
    let near = Meters(1.0)..Meters(5.0);
    println!("{} {} {}", near.contains(&2.5), near.contains(&5.0), near.contains(&f64::NAN));
}
