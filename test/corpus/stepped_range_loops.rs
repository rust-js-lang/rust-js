// A range's `step_by(n)` in a `for`, `for (let i = a; i < b; i += n)`, as
// `a..b`'s is a counting `for`, not `for (const i of $lazyRange(a, b, n))`;
// its index below a slice's length where `b` is it, `v[i]`; and a stepped
// index to a slice of items that may be `None` themselves, `let Some(x) =
// v.get(i) else { break };`, counted to its length too, `const x = v[i];`
// (rd260).

fn pairs(v: &[u32]) -> u32 {
    let mut total = 0;
    for i in (0..v.len()).step_by(2) {
        total += v[i];
    }
    total
}

fn cleared(v: &mut Vec<u32>) -> u32 {
    let mut total = 0;
    for i in (1..v.len()).step_by(3) {
        total += v[i];
        v[i] = 0;
        if total > 100 {
            break;
        }
    }
    total
}

fn evens_up_to(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for i in (0..=n).step_by(2) {
        if i % 3 == 0 {
            continue;
        }
        found.push(i);
    }
    found
}

fn down(n: u32) -> Vec<u32> {
    let mut found = Vec::new();
    for i in (0..n).rev().step_by(3) {
        found.push(i);
    }
    found
}

fn each(n: usize) -> Vec<Box<dyn Fn() -> usize>> {
    let mut made: Vec<Box<dyn Fn() -> usize>> = Vec::new();
    for i in (n..n + 7).step_by(1) {
        made.push(Box::new(move || i * 10));
    }
    made
}

fn options(v: &[Option<u32>]) -> u32 {
    let mut total = 0;
    for i in (0..).step_by(2) {
        let Some(x) = v.get(i) else { break };
        total += x.unwrap_or(100);
    }
    total
}

fn main() {
    println!("{} {}", pairs(&[1, 2, 3, 4, 5]), pairs(&[]));
    let mut v = vec![5, 6, 7, 8, 90, 10, 11, 12];
    println!("{} {:?}", cleared(&mut v), v);
    println!("{:?} {:?}", evens_up_to(10), evens_up_to(0));
    println!("{:?} {:?}", down(10), down(0));
    println!("{:?}", each(3).iter().map(|f| f()).collect::<Vec<_>>());
    println!(
        "{} {} {}",
        options(&[Some(1), None, None, Some(2), Some(3)]),
        options(&[None]),
        options(&[])
    );
}
