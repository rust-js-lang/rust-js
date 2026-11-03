// A `match` of an `Option<bool>` whose arms name `Some(false)` and
// `Some(true)` is a `switch` of its value, `case false:` and `case true:`,
// as React's updateClassComponent switches on `shouldError(..)` (#545):
// a last `None => {}` or `_ => {}` is no case, as `null` matches none, and
// one that does something is the `default`, which only `null` reaches,
// but for a loop's `break`, which in a `switch` would leave the `switch`.
// Of an `Option<u32>`, whose `Some`s name a few values, `None` is an `if`.
fn should_error(n: u32) -> Option<bool> {
    if n == 0 { None } else { Some(n > 5) }
}

fn level(n: u32) -> Option<u32> {
    if n == 0 { None } else { Some(n % 3) }
}

pub fn update(n: u32, log: &mut Vec<u32>) {
    match should_error(n) {
        Some(false) => {
            log.push(1);
            log.push(10);
        }
        Some(true) => {
            log.push(2);
            log.push(20);
        }
        None => {}
    }
    log.push(3);
}

pub fn update_none(n: u32, log: &mut Vec<u32>) {
    match should_error(n) {
        Some(true) => {
            log.push(2);
        }
        Some(false) => {
            log.push(1);
        }
        None => {
            log.push(4);
        }
    }
    log.push(3);
}

pub fn returned(n: u32) -> u32 {
    match should_error(n) {
        Some(true) => 1,
        Some(false) => 3,
        _ => 2,
    }
}

pub fn guarded(n: u32, on: bool) -> u32 {
    let mut total = 0;
    match should_error(n) {
        Some(false) if on => total += 1,
        Some(true) => total += 10,
        _ => total += 100,
    }
    total
}

pub fn in_a_loop(ns: &[u32]) -> u32 {
    let mut total = 0;
    for &n in ns {
        match should_error(n) {
            Some(false) => total += 1,
            Some(true) => {
                if total > 4 {
                    break;
                }
                total += 10;
            }
            None => continue,
        }
        total *= 2;
    }
    total
}

pub fn until_none(ns: &[u32]) -> u32 {
    let mut total = 0;
    for &n in ns {
        match should_error(n) {
            Some(false) => total += 1,
            Some(true) => total += 10,
            None => break,
        }
        total *= 2;
    }
    total
}

pub fn levels(n: u32, log: &mut Vec<u32>) {
    match level(n) {
        Some(1) => {
            log.push(1);
        }
        Some(2) => {
            log.push(2);
        }
        None => {
            log.push(4);
        }
        _ => {}
    }
    log.push(3);
}

fn main() {
    let mut log = Vec::new();
    for n in [0, 3, 9] {
        update(n, &mut log);
        update_none(n, &mut log);
        levels(n, &mut log);
        println!("{n}: {} {} {}", returned(n), guarded(n, true), guarded(n, false));
    }
    println!("{log:?}");
    println!("{}", in_a_loop(&[1, 0, 7, 2, 8, 9]));
    println!("{}", in_a_loop(&[0, 0, 6]));
    println!("{}", until_none(&[1, 7, 0, 9]));
}
