//@ library-differs
// An integer bound by another's value, `i < n`, as by a slice's length: `n
// - i` can't wrap, nor `n - i - 1` below `n`, so each is JS's own. Where
// it can, or `n` changed since, each wraps as Rust's does.

fn coefficients(line: u32) -> Vec<u32> {
    let mut found = Vec::new();
    let size = line + 1;
    for k in 1..size {
        found.push(line - k + 1);
    }
    found
}

fn reversed(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for i in 0..n {
        found.push(n - i - 1);
    }
    found
}

fn past_the_end(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for i in 0..n + 2 {
        found.push(n - i);
    }
    found
}

fn tested(n: u32, i: u32) -> u32 {
    if i < n { n - i - 1 } else { i - n }
}

fn changed_since(mut n: u32, i: u32) -> u32 {
    let end = n + 1;
    n = 0;
    if i < end { n - i } else { 7 }
}

fn counted(mut n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < n {
        found.push(n - i);
        n -= 1;
        i += 1;
    }
    found
}

fn stepped(n: u8) -> Vec<u8> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < n {
        found.push(n - i);
        i += 1;
        i += 1;
    }
    found
}

fn shrunk(mut n: u32, i: u32) -> u32 {
    if n > 0 {
        n -= 1;
        if i < n {
            return n - i - 1 - 1;
        }
    }
    0
}

fn sum_below(xs: &[u32], n: usize) -> u32 {
    let mut sum = 0;
    let mut i = 0;
    while i < xs.len() {
        if i < n {
            sum += xs[i];
        }
        i += 1;
    }
    sum
}

fn two_past(n: u8, i: u8) -> u8 {
    if i < n { i + 1 + 1 } else { 0 }
}

/// A number the compiler can't see, as a program's input is.
fn input<T: std::str::FromStr>(text: &str) -> T {
    text.parse().ok().expect("a number")
}

fn main() {
    println!("{:?}", coefficients(input("0")));
    println!("{:?}", coefficients(input("4")));
    println!("{:?}", coefficients(input("4294967295")));
    println!("{:?}", reversed(input("4")));
    println!("{:?}", reversed(input("0")));
    println!("{:?}", past_the_end(input("2")));
    println!("{:?}", past_the_end(input("4294967294")).len());
    println!(
        "{} {} {}",
        tested(input("5"), input("2")),
        tested(input("5"), input("5")),
        tested(input("5"), input("9"))
    );
    println!(
        "{} {}",
        changed_since(input("5"), input("3")),
        changed_since(input("5"), input("0"))
    );
    println!("{:?}", counted(input("5")));
    println!("{:?}", stepped(input("5")));
    println!("{:?}", stepped(input("6")));
    println!("{} {}", shrunk(input("5"), input("3")), shrunk(input("9"), input("1")));
    println!("{}", sum_below(&[1, 2, 3, 4], input("2")));
    println!(
        "{} {}",
        two_past(input("255"), input("254")),
        two_past(input("9"), input("3"))
    );
}
