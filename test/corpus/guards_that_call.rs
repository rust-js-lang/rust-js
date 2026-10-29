// A `match` whose guard calls, `[n, ..] if below(*n)`: the chain of its
// arms, each call made where the tree makes it and no other time, and
// what's read after it what the guard can't change, its `match`'s tests
// (#221). `row.length >= 1 && below(row[0])`, not a labeled block the
// guard's `break` leaves. A guard that changes what a later guard reads
// keeps MIR's tree; a value a guard binds that its arm reads, its `const`.

use std::cell::Cell;

fn odd(n: u32) -> bool {
    println!("odd {n}");
    n % 2 == 1
}

fn small(n: i32) -> bool {
    println!("small {n}");
    n.abs() < 3
}

fn big(n: i32) -> bool {
    println!("big {n}");
    n > 100
}

// Its guard's binding read by the arm too.
fn firsts(rows: &[&[u32]]) -> Vec<u32> {
    let mut seen = Vec::new();
    for row in rows {
        let level = match row {
            [n, ..] if odd(*n) => *n,
            [_, m] => *m + 1,
            _ => 0,
        };
        seen.push(level);
    }
    seen
}

// Guards of two calls, of one, and a test between.
fn sorted(pairs: &[(i32, i32)]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for &(a, b) in pairs {
        out.push(match (a, b) {
            (0, _) => "zero",
            (x, y) if small(x) && small(y) => "small",
            (x, _) if x < 0 => "negative",
            (_, y) if big(y) => "big",
            _ => "other",
        });
    }
    out
}

// A guard that calls, then one that reads what the call changed.
fn counted(values: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for &v in values {
        let mut calls = 0;
        let mut bump = |n: u32| {
            calls += 1;
            n > 2
        };
        let kind = match v {
            0 => 0,
            n if bump(n) => 1,
            _ if calls > 0 => 2,
            _ => 3,
        };
        out.push(kind * 10 + calls);
    }
    out
}

// Of two `|`'s alternatives, each tested by the guard in turn.
fn either(pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let level = match (a, b) {
            (1, x) | (x, 1) if odd(x) => 1,
            (2, 2) => 2,
            (3, _) | (_, 3) => 3,
            _ => 5,
        };
        seen.push(level);
    }
    seen
}

// Of each alternative's binding the arm reads too, given where it is.
fn either_read(pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let level = match (a, b) {
            (1, x) | (x, 1) if odd(x) => x * 10,
            (2, 2) => 2,
            _ => 5,
        };
        seen.push(level);
    }
    seen
}

// A guard of no binding, the same call of each alternative: made again
// where the first fails.
fn sums(pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let level = match (a, b) {
            (1, _) | (_, 1) if odd(a + b) => 1,
            (2, 2) => 2,
            _ => 5,
        };
        seen.push(level);
    }
    seen
}

// A guard whose call changes what it then reads: MIR's tree.
fn marked(rows: &[&[u32]]) -> Vec<u32> {
    let mut seen = Vec::new();
    for row in rows {
        let done = Cell::new(false);
        let mark = |n: u32| {
            println!("mark {n}");
            done.set(n > 4);
            n % 2 == 1
        };
        let mut level = 4;
        match row {
            [1, ..] | [.., 1] => level = 1,
            [2, 2] => level = 2,
            [n, ..] if mark(*n) && done.get() => level = 3,
            _ => level += 1,
        }
        seen.push(level);
    }
    seen
}

// A guard whose arm does what the last does: called still, as each way
// to that arm is the tree's.
fn unmarked(rows: &[&[u32]]) -> Vec<u32> {
    let mut seen = Vec::new();
    for row in rows {
        let mark = |n: u32| {
            println!("mark {n}");
            n % 2 == 1
        };
        let mut level = 0;
        match row {
            [1, ..] | [.., 1] => level = 1,
            [2, 2] => continue,
            [n, ..] if mark(*n) => level = 5,
            _ => level = 5,
        }
        seen.push(level);
    }
    seen
}

// A guard that reads the value a closure reads.
fn below(rows: &[&[u32]]) -> Vec<u32> {
    let mut seen = Vec::new();
    for row in rows {
        let mut level = 4;
        let under = |n: u32| {
            println!("under {n}");
            n < level
        };
        match row {
            [1, ..] | [.., 1] => level = 1,
            [2, 2] => level = 2,
            [n, ..] if under(*n) => level = 3,
            _ => level = 5,
        }
        seen.push(level);
    }
    seen
}

fn main() {
    println!("{:?}", firsts(&[&[], &[3], &[4, 5], &[4, 5, 6], &[2]]));
    println!(
        "{:?}",
        sorted(&[(0, 9), (1, 2), (1, 5), (-1, 5), (-7, 500), (7, 500), (5, 5), (2, -200)])
    );
    println!("{:?}", counted(&[0, 1, 2, 3, 9]));
    println!(
        "{:?}",
        either(&[
            (1, 1),
            (1, 2),
            (2, 1),
            (3, 1),
            (2, 2),
            (3, 4),
            (4, 3),
            (1, 3),
            (3, 3),
            (4, 4)
        ])
    );
    let pairs = [(1, 1), (1, 2), (2, 1), (3, 1), (2, 2), (1, 4), (4, 1), (5, 5)];
    println!("{:?}", either_read(&pairs));
    println!("{:?}", sums(&pairs));
    println!("{:?}", marked(&[&[], &[1], &[2, 2], &[3], &[5], &[6, 2], &[7, 3, 9]]));
    println!("{:?}", unmarked(&[&[], &[1], &[2, 2], &[3], &[6, 2]]));
    println!(
        "{:?}",
        below(&[&[], &[1], &[2, 1], &[2, 2], &[2, 3], &[3], &[5, 6, 7], &[9]])
    );
}
