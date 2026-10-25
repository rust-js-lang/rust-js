// An integer's test `!= 0` is its truth, `if (lanes)`, never `NaN` (ADR
// 0896, amended): of each width, and of a wrap; its `== 0` stays `=== 0`,
// a remainder's test stays as written, and a float's stays `x !== 0`,
// `NaN` being unequal to `0` but falsy.

fn pick(lanes: i32) -> &'static str {
    if lanes != 0 { "some" } else { "none" }
}

fn count(mut lanes: i32) -> u32 {
    let mut n = 0;
    while lanes != 0 {
        lanes &= lanes - 1;
        n += 1;
    }
    n
}

fn widths(a: i8, b: u8, c: u32, d: usize, e: i16) -> [bool; 5] {
    let mut seen = [false; 5];
    if a != 0 {
        seen[0] = true;
    }
    if b == 0 {
        seen[1] = true;
    }
    if c != 0 {
        seen[2] = true;
    }
    if d == 0 {
        seen[3] = true;
    }
    if e != 0 {
        seen[4] = true;
    }
    seen
}

fn arithmetic(n: i32, m: i32) -> u32 {
    let mut seen = 0;
    if n % 3 == 0 {
        seen |= 1;
    }
    if n.wrapping_add(m) != 0 {
        seen |= 2;
    }
    if n - m == 0 {
        seen |= 4;
    }
    if m != 0 && n % m != 0 {
        seen |= 8;
    }
    if n.wrapping_sub(m) != 0 {
        seen |= 16;
    }
    seen
}

fn float(x: f64) -> bool {
    if x != 0.0 {
        return true;
    }
    false
}

fn main() {
    for lanes in [0, 1, -1, i32::MIN, 0b1010] {
        println!("{} {}", pick(lanes), count(lanes));
    }
    println!("{:?}", widths(0, 0, 0, 0, 0));
    println!("{:?}", widths(-128, 255, u32::MAX, usize::MAX, -1));
    for (n, m) in [(0, 0), (3, -3), (7, 2), (i32::MAX, 1), (-6, -6)] {
        println!("{}", arithmetic(n, m));
    }
    for x in [0.0, -0.0, f64::NAN, 0.5, f64::INFINITY] {
        println!("{}", float(x));
    }
}
