// `peekable()` of a lazy iterator, a chain of `map` or `filter`: it steps
// its source only when asked, as Rust's `Peekable` does, so its closures
// run in Rust's order; `peek`, `next_if` and `next_if_eq` of it (ADR 0164).
fn tokens(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = src.chars().filter(|c| !c.is_whitespace()).peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut number = c.to_string();
            while let Some(&d) = chars.peek() {
                if !d.is_ascii_digit() {
                    break;
                }
                number.push(d);
                chars.next();
            }
            out.push(number);
        } else {
            out.push(c.to_string());
        }
    }
    out
}

fn main() {
    let mut seen = Vec::new();
    let mut it = (1..6)
        .map(|x| {
            seen.push(x);
            x * 10
        })
        .peekable();
    let p = it.peek().copied();
    let q = it.peek().copied();
    let first = it.next();
    let taken = it.next_if(|&x| x == 20);
    let refused = it.next_if(|&x| x == 99);
    let eq = it.next_if_eq(&30);
    let rest: Vec<i32> = it.collect();
    println!("{p:?} {q:?} {first:?} {taken:?} {refused:?} {eq:?} {rest:?} {seen:?}");
    println!("{:?}", tokens("12 + 3*45"));
    let mut empty = std::iter::empty::<u8>().map(|x| x + 1).peekable();
    // Of items that look like `None`: `Some(None)`, not `None`.
    let mut maybes = [Some(1), None].into_iter().map(|x| x.map(|n| n + 1)).peekable();
    let a = maybes.next();
    let b = maybes.peek().copied();
    let c = maybes.peek().copied();
    println!("{a:?} {b:?} {c:?} {:?}", maybes.next());
    let peeked = empty.peek().copied();
    println!("{:?} {:?}", peeked, empty.next());
}
