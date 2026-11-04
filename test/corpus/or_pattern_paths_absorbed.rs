// A match's arm reached by paths alike but for one value's test is one
// path, that value's test of either's values (#631): `(0, 0) => ..,
// (1, _) | (_, 1) => ..` is `if (a === 0 && b === 0) { .. } else if (a ===
// 1 || b === 1) {`, not MIR's labeled block. A path whose or-pattern none
// of its alternatives can hold, of what it tests first, is none. Each test
// is a pure read of what's matched, and the scrutinee's calls run once, as
// the log shows: each input takes the arm native Rust takes.
pub fn pair(a: u32, b: u32, log: &mut Vec<u32>) -> u32 {
    let value = match (a, b) {
        (0, 0) => {
            log.push(0);
            1
        }
        (1, _) | (_, 1) => 2,
        (2, 3) | (3, 2) => {
            log.push(a);
            7
        }
        _ => a + b,
    };
    log.push(value);
    value
}

pub fn first_two(a: u32, b: u32) -> u32 {
    match (a, b) {
        (2, 0) => 1,
        (1, _) | (_, 1) => 2,
        (2, 3) | (3, 2) => 3,
        _ => 4,
    }
}

pub fn triple(a: u8, b: u8, c: u8) -> u8 {
    match (a, b, c) {
        (0, 0, 0) => 1,
        (1, _, _) | (_, 1, _) | (_, _, 1) => 2,
        (2, 2, _) | (_, 2, 2) => 3,
        _ => 4,
    }
}

pub fn optional(x: Option<u32>, n: u32) -> u32 {
    match (x, n) {
        (Some(0), 0) => 1,
        (None, _) | (_, 1) => 2,
        (Some(2), 2) | (Some(3), 3) => 3,
        _ => 4,
    }
}

pub enum Shape {
    Circle(u32),
    Rect(u32, u32),
}

pub fn shaped(shape: &Shape, k: u32) -> u32 {
    match (shape, k) {
        (Shape::Circle(0), 0) => 1,
        (Shape::Circle(1), _) | (_, 1) => 2,
        (Shape::Rect(2, 2), _) | (Shape::Circle(2), 2) => 3,
        _ => 4,
    }
}

pub fn sliced(xs: &[u32]) -> u32 {
    match xs {
        [0, 0] => 1,
        [1, _] | [_, 1] => 2,
        [2, 3] | [3, 2] => 3,
        _ => 4,
    }
}

fn logged(log: &mut Vec<u32>, value: u32) -> u32 {
    log.push(100 + value);
    value
}

pub fn called(a: u32, b: u32, log: &mut Vec<u32>) -> u32 {
    match (logged(log, a), logged(log, b)) {
        (0, 0) => 1,
        (1, _) | (_, 1) => 2,
        (2, 3) | (3, 2) => 3,
        _ => 4,
    }
}

fn main() {
    let mut log = Vec::new();
    let mut out = Vec::new();
    for a in 0..5 {
        for b in 0..5 {
            out.push(pair(a, b, &mut log));
            out.push(called(a, b, &mut log));
            out.push(first_two(a, b));
            out.push(sliced(&[a, b]));
            out.push(optional(if a == 4 { None } else { Some(a) }, b));
            out.push(shaped(&Shape::Circle(a), b));
            out.push(shaped(&Shape::Rect(a, b), a));
            for c in 0..4 {
                out.push(u32::from(triple(a as u8, b as u8, c)));
            }
        }
    }
    out.push(sliced(&[]));
    out.push(sliced(&[1]));
    out.push(sliced(&[1, 2, 3]));
    println!("{out:?}");
    println!("{log:?}");
}
