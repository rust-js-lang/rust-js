// A match of three or more values whose or-pattern names one each,
// `(1, _, _) | (_, 1, _) | (_, _, 1)`, is the chain as written (#673): a
// path whose test's failing says another path of its arm, `a !== 2 && b
// === 2 && c === 2` after `a === 2 && b === 2`, needs no `a !== 2`; what
// an arm before it says failing, `a !== 1`, `b !== 1` and `c !== 1`, an
// arm after needs no test of; and `a !== 2 || a === 2` is no test. A test
// whose failing would read what throws stays, `x != null` before `x[0]`,
// and a value's `NaN` takes the arm native Rust takes. `present`,
// `absent`, `spread` and `rows` are generated matches whose JS a wrong rule
// makes wrong. Each input below takes the arm native Rust takes.
pub fn triple(a: u8, b: u8, c: u8) -> u8 {
    match (a, b, c) {
        (0, 0, 0) => 1,
        (1, _, _) | (_, 1, _) | (_, _, 1) => 2,
        (2, 2, _) | (_, 2, 2) => 3,
        _ => 4,
    }
}

pub fn quad(a: u8, b: u8, c: u8, d: u8) -> u8 {
    match (a, b, c, d) {
        (0, 0, 0, 0) => 1,
        (1, _, _, _) | (_, 1, _, _) | (_, _, 1, _) | (_, _, _, 1) => 2,
        (2, 2, _, _) | (_, _, 2, 2) => 3,
        _ => 4,
    }
}

pub fn first_two(a: u32, b: u32) -> u32 {
    match (a, b) {
        (2, 0) => 1,
        (1, _) | (_, 1) => 2,
        (2, 3) | (3, 2) => 3,
        _ => 4,
    }
}

pub fn guarded(x: Option<(u8, u8)>, k: u8) -> u8 {
    match (x, k) {
        (None, 5) | (Some((1, _)), 5) => 1,
        (Some((_, 1)), _) | (None, 1) => 2,
        (Some((2, 2)), _) | (_, 2) => 3,
        _ => 4,
    }
}

pub fn listed(xs: &[(u8, u8)], k: u8) -> u8 {
    match (xs, k) {
        ([], 0) => 1,
        ([(1, _), ..], _) | ([_, (_, 1), ..], _) | (_, 1) => 2,
        ([(2, 2)], _) | ([_, (2, _)], 2) => 3,
        _ => 4,
    }
}

pub fn measured(x: f64, n: u8) -> u8 {
    match (x, n) {
        (_, 0) => 1,
        (1.0, _) | (_, 1) => 2,
        (2.0, 2) | (_, 3) => 3,
        _ => 4,
    }
}

pub fn present(x: Option<(u8, u8)>, n: u8) -> u8 {
    match (x, n) {
        (Some((0, _)), _) | (Some(_), 1) => 1,
        (Some((2, _)), _) => 2,
        (_, 2) => 3,
        _ => 4,
    }
}

pub fn absent(x: Option<(u8, u8)>, n: u8) -> u8 {
    match (x, n) {
        (_, 1) => 1,
        (Some((_, 1)), _) | (None, _) | (Some((1, 2)), _) => 2,
        _ => 3,
    }
}

pub fn spread(a: u8, b: u8, c: u8, d: u8) -> u8 {
    match (a, b, c, d) {
        (1, _, 1, _) => 1,
        (_, 2, 0, 2) | (2, 2, _, _) | (1, _, _, 0) | (_, 2, 0, _) => 2,
        (2, 1, 2, 0) | (_, 2, 2, _) | (_, _, _, 2) => 3,
        (_, 1, 1, 0) | (1, _, _, _) | (_, _, _, 0) | (2, _, _, _) => 4,
        _ => 5,
    }
}

pub fn rows(a: u8, b: u8, c: u8) -> u8 {
    match (a, b, c) {
        (1, 2, 0) | (1, _, 1) => 1,
        (_, 0, _) => 2,
        (1, _, _) | (_, 2, _) | (0, _, 2) => 3,
        _ => 4,
    }
}

fn main() {
    let mut out = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            out.push(u32::from(first_two(a, b)));
            for c in 0..4 {
                out.push(u32::from(triple(a as u8, b as u8, c)));
                for d in 0..4 {
                    out.push(u32::from(quad(a as u8, b as u8, c, d)));
                }
            }
        }
    }
    let options = [None, Some((0, 0)), Some((1, 0)), Some((0, 1)), Some((2, 2)), Some((2, 1)), Some((1, 1))];
    for x in options {
        for k in 0..7 {
            out.push(u32::from(guarded(x, k)));
        }
    }
    let lists: [&[(u8, u8)]; 9] = [
        &[],
        &[(1, 0)],
        &[(0, 1)],
        &[(0, 0), (0, 1)],
        &[(2, 2)],
        &[(0, 0), (2, 0)],
        &[(0, 0), (2, 0), (0, 0)],
        &[(2, 2), (0, 0)],
        &[(0, 0), (0, 0), (0, 1)],
    ];
    for xs in lists {
        for k in 0..4 {
            out.push(u32::from(listed(xs, k)));
        }
    }
    let pairs = [None, Some((0, 0)), Some((0, 1)), Some((1, 2)), Some((2, 1)), Some((1, 1)), Some((3, 3))];
    for x in pairs {
        for n in 0..4 {
            out.push(u32::from(present(x, n)));
            out.push(u32::from(absent(x, n)));
        }
    }
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                out.push(u32::from(rows(a, b, c)));
                for d in 0..4 {
                    out.push(u32::from(spread(a, b, c, d)));
                }
            }
        }
    }
    for x in [0.0, 1.0, 2.0, -1.0, f64::NAN, f64::INFINITY] {
        for n in 0..5 {
            out.push(u32::from(measured(x, n)));
        }
    }
    println!("{out:?}");
}
