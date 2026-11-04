// A range's `is_empty()` is `!(start < end)`, of `a..=b` `!(start <= end)`,
// by its bounds' `PartialOrd`: JS's own `<` of numbers, by code point of
// `char`s and strings, field by field of a struct, a tuple or an array, and a
// hand-written `lt` or `partial_cmp`, where JS's `<` of two arrays compares
// their text, `"10" < "9"`, and of two objects nothing.

use std::cmp::Ordering;

#[derive(Debug, PartialEq, PartialOrd)]
struct Ticket(u32);

#[derive(Debug, PartialEq, PartialOrd)]
struct Point {
    x: i32,
    y: i32,
}

// Ordered backwards, by a `partial_cmp` of its own.
#[derive(Debug, PartialEq)]
struct Countdown(u8);

impl PartialOrd for Countdown {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

// Ordered by a `lt` and `le` of its own, which std's `<` and `<=` call.
#[derive(Debug, PartialEq)]
struct Loud(i32);

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }

    fn lt(&self, other: &Self) -> bool {
        println!("lt {} {}", self.0, other.0);
        self.0 < other.0
    }

    fn le(&self, other: &Self) -> bool {
        println!("le {} {}", self.0, other.0);
        self.0 <= other.0
    }
}

fn empty<T: PartialOrd>(range: std::ops::Range<T>) -> bool {
    range.is_empty()
}

fn main() {
    println!("{}", (Ticket(10)..Ticket(9)).is_empty());
    println!("{}", (Ticket(9)..Ticket(10)).is_empty());
    println!("{}", (Ticket(10)..=Ticket(9)).is_empty());
    println!("{}", (Point { x: 1, y: 5 }..Point { x: 2, y: 0 }).is_empty());
    println!("{}", (Point { x: 2, y: 0 }..Point { x: 1, y: 5 }).is_empty());
    println!("{}", ((10, 0)..(9, 0)).is_empty());
    println!("{}", ([10, 0]..[9, 0]).is_empty());
    println!("{}", ('\u{ffff}'..'\u{10000}').is_empty());
    println!("{}", ('\u{10000}'..='\u{ffff}').is_empty());
    let (low, high) = (String::from("\u{ffff}"), String::from("\u{10000}"));
    println!("{}", (low.clone()..high.clone()).is_empty());
    println!("{}", (high..=low).is_empty());
    println!("{}", (Countdown(3)..Countdown(1)).is_empty());
    println!("{}", (Countdown(1)..=Countdown(3)).is_empty());
    println!("{}", (Loud(1)..Loud(2)).is_empty());
    println!("{}", (Loud(2)..=Loud(2)).is_empty());
    println!("{}", empty(Ticket(10)..Ticket(9)));
    println!("{}", empty('\u{ffff}'..'\u{10000}'));
    println!("{}", empty(3..2));
    let nan = std::hint::black_box(f64::NAN);
    println!("{} {}", (nan..1.0).is_empty(), (0.0..=nan).is_empty());
    println!("{} {}", (1..4).is_empty(), (4..=4).is_empty());
}
