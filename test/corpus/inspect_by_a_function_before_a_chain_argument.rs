// A chain's argument is made before the chain is stepped, as Rust's
// arguments are: `items.inspect(see).chain(vec![probe(9)])` collected runs
// `probe(9)` before `see` sees an item, whether `see` is a function or a
// closure. Its JS spreads what's chained, `[...it, probe(9)]`, which steps
// `it` first: a function's adapter is plain data in Rust, but spreading it
// runs what it calls, so nothing that acts is moved past it. It ran `see 0`
// before `probe 9` (#1015).
fn probe(n: u32) -> u32 {
    println!("probe {n}");
    n
}

fn see(n: &u32) {
    println!("see {n}");
}

fn main() {
    let by_function: Vec<u32> = vec![probe(0)].into_iter().inspect(see).chain(vec![probe(9)]).collect();
    println!("{by_function:?}");
    let by_closure: Vec<u32> = vec![probe(1)]
        .into_iter()
        .inspect(|n| see(n))
        .chain(vec![probe(8)])
        .collect();
    println!("{by_closure:?}");
}
