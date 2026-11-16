// A `scan()` of an iterator held apart is std's compiled `Scan`, which
// gives its closure the state's `&mut`, a box: a closure that takes the
// state as JS holds it, an object, reads and writes it through the box in
// its own body, never a copy, so a write, the whole state replaced too, is
// the box's, and a closure in it reads the box within the call, as Rust's
// borrow ends there (#2008).
#[derive(Debug)]
struct State {
    n: u32,
    log: Vec<u32>,
}

fn main() {
    let v = [1u32, 2, 3];
    let reads_of = v.iter();
    let reads = reads_of.scan(State { n: 10, log: vec![] }, |s, x| Some(s.n + x));
    println!("{:?}", reads.collect::<Vec<_>>());
    let writes_of = v.iter();
    let writes = writes_of.scan(State { n: 0, log: vec![] }, |s, x| {
        s.log.push(*x);
        *s = State { n: s.n + x, log: s.log.clone() };
        Some(*x)
    });
    println!("{:?}", writes.collect::<Vec<_>>());
    let both_of = v.iter();
    let both = both_of.scan(State { n: 1, log: vec![] }, |s, x| {
        let before = s.n;
        s.n *= x + 1;
        s.log.push(before);
        if s.n > 20 { None } else { Some((before, s.n, s.log.len())) }
    });
    println!("{:?}", both.collect::<Vec<_>>());
    let nested_of = v.iter();
    let nested = nested_of.scan(State { n: 5, log: vec![] }, |s, x| {
        let add = |y: &u32| s.n + y;
        let sum = add(x);
        s.n = sum;
        Some(sum)
    });
    println!("{:?}", nested.collect::<Vec<_>>());
}
