// A thread-local whose initializer makes a table and gives it, `{ let mut
// days = HashMap::new(); for .. { days.insert(..); } days }`: the module's
// own statements, `const regionDays = new Map(); for (..) { .. }`, as the
// vercel-ui SDK writes it (G1), read by the module's functions and, in
// another module, through its function. One whose initializer makes two
// names at its top stays a closure called where it's made, as its other
// name would be the module's; so does one that gives a value early, whose
// `return` no module's top may have.

use std::cell::Cell;
use std::collections::HashMap;

thread_local! {
    static REGION_DAYS: HashMap<&'static str, u32> = {
        let mut region_days = HashMap::new();
        for (day, regions) in [(0, "US CA JP"), (5, "MV"), (6, "EG SA")] {
            for region in regions.split(' ') {
                region_days.insert(region, day);
            }
        }
        region_days
    };
    static SPAN: (u32, u32) = {
        let low = "2".parse().unwrap_or(0);
        let high = low * 10;
        (low, high)
    };
    static CALLS: Cell<u32> = const { Cell::new(0) };
    static EARLY: Vec<u32> = {
        let given: Vec<u32> = "1 2".split(' ').map(|s| s.parse().unwrap_or(0)).collect();
        if given.len() > 1 {
            let mut fewer = vec![9];
            fewer.push(given[0]);
            return fewer;
        }
        given
    };
    static COUNTED: Vec<u32> = {
        let mut counted = Vec::new();
        let base = "3".parse().unwrap_or(1);
        for i in 0..base {
            counted.push(i * 2);
        }
        counted
    };
}

fn first_day(region: &str) -> u32 {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    REGION_DAYS.with(|region_days| region_days.get(region).copied().unwrap_or(1))
}

mod weeks {
    use std::collections::HashMap;

    thread_local! {
        static NAMES: HashMap<u32, String> = {
            let mut names = HashMap::new();
            for (i, name) in ["sun", "mon", "tue"].iter().enumerate() {
                names.insert(i as u32, name.to_uppercase());
            }
            names
        };
    }

    pub fn name(day: u32) -> String {
        NAMES.with(|names| names.get(&day).cloned().unwrap_or_else(|| "?".to_string()))
    }
}

fn main() {
    for region in ["US", "MV", "EG", "DE"] {
        let day = first_day(region);
        println!("{region}: {day} {}", weeks::name(day));
    }
    println!("{}", REGION_DAYS.with(|region_days| region_days.len()));
    println!("{:?} {}", SPAN.with(|span| *span), CALLS.with(Cell::get));
    println!("{:?} {:?}", EARLY.with(Clone::clone), COUNTED.with(Clone::clone));
}
