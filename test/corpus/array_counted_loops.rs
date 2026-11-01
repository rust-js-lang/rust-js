// ADR 1042, the user's decision: a `for` over an array's items is an index
// loop, `for (let i = 0; i < v.length; i++) { const x = v[i]; .. }`, as React
// DOM's Fizz runtime writes it; `j` inside one of `i`, sibling loops each `i`.
// A map's items, a string's chars and a `Vec` the body gives another value
// stay `for…of`.

use std::collections::BTreeMap;

fn borrowed(v: &[u32]) -> u32 {
    let mut total = 0;
    for x in v {
        total += x;
    }
    total
}

fn owned(v: Vec<String>) -> usize {
    let mut n = 0;
    for s in v {
        n += s.len();
    }
    n
}

fn nested(rows: &[Vec<u32>]) -> u32 {
    let mut total = 0;
    for row in rows {
        for &x in row.iter() {
            total += x;
        }
    }
    for row in rows {
        total += row.len() as u32;
    }
    total
}

fn pairs(v: &[(u32, u32)]) -> u32 {
    let mut total = 0;
    for &(a, b) in v {
        if a == 0 {
            continue;
        }
        total += a * b;
    }
    total
}

fn reassigned(mut v: Vec<u32>) -> u32 {
    let mut total = 0;
    for x in v {
        v = vec![x; 3];
        total += x + v.len() as u32;
    }
    total
}

fn text(s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        if c == '😀' {
            n += 1;
        }
    }
    n
}

fn map(m: &BTreeMap<u32, u32>) -> u32 {
    let mut total = 0;
    for (k, v) in m {
        total += k * v;
    }
    total
}

fn main() {
    println!("{}", borrowed(&[1, 2, 3]));
    println!("{}", owned(vec!["a".to_string(), "bcd".to_string()]));
    println!("{}", nested(&[vec![1, 2], vec![], vec![3]]));
    println!("{}", pairs(&[(0, 5), (2, 3), (4, 1)]));
    println!("{}", reassigned(vec![1, 2, 3]));
    println!("{}", text("a😀b😀"));
    println!("{}", map(&BTreeMap::from([(1, 2), (3, 4)])));
}
