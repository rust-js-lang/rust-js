// Branches that give the same value: the value alone where the test does
// nothing, `v = 1.5`, `r._0` of either variant's payload; an `if` still
// where the test calls, which runs as written, `if (mark(n)) { return 5; }
// return 5;`, not `mark(n) ? 5 : 5`; and `0.0` and `-0.0` are two values.

fn mark(n: u32) -> bool {
    println!("mark {n}");
    n % 2 == 1
}

fn returned(n: u32) -> u32 {
    if mark(n) { 5 } else { 5 }
}

fn assigned(n: u32) -> u32 {
    let mut level = 0;
    if n > 0 {
        if mark(n) {
            level = 5;
        } else {
            level = 5;
        }
    }
    level
}

fn early(n: u32) -> u32 {
    if mark(n) {
        return 5;
    }
    5
}

fn joined(n: u32) -> u32 {
    let level = if n > 2 && mark(n) { 5 } else { 5 };
    level + 1
}

fn signed(n: u32) -> f64 {
    if n > 3 { 0.0 } else { -0.0 }
}

fn same(n: u32) -> f64 {
    let v = if n > 3 { 1.5 } else { 1.5 };
    v * 2.0
}

fn payload(r: Result<u32, u32>) -> u32 {
    let n = match r {
        Ok(n) | Err(n) => n,
    };
    n + 1
}

fn main() {
    println!("{} {}", returned(3), returned(4));
    println!("{} {}", assigned(0), assigned(7));
    println!("{} {} {} {}", early(1), early(2), joined(3), joined(1));
    println!("{} {}", 1.0 / signed(5), 1.0 / signed(1));
    println!("{}", same(2));
    println!("{} {}", payload(Ok(1)), payload(Err(2)));
}
