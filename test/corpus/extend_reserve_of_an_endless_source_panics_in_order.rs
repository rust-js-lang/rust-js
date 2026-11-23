// An endless source unzipped into sides generic code is given: the pair's
// `extend_reserve` reserves its first side first, hashbrown's room check
// of a `HashSet`, `Hash table capacity overflow`, or a `Vec`'s,
// `capacity overflow` (#2414).
use std::collections::HashSet;

fn endless<A: Default + Extend<u32>>() -> (A, Vec<u32>) {
    (0u32..).map(|n| (n, n)).unzip()
}

fn endless_after<A: Default + Extend<u32>>() -> (Vec<u32>, A) {
    (0u32..).map(|n| (n, n)).unzip()
}

fn message(r: Result<usize, Box<dyn std::any::Any + Send>>) -> String {
    match r {
        Ok(n) => format!("ok {n}"),
        Err(e) => e
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| e.downcast_ref::<String>().cloned())
            .unwrap_or_default(),
    }
}

fn main() {
    println!(
        "{}",
        message(std::panic::catch_unwind(|| endless::<HashSet<u32>>().1.len()))
    );
    println!(
        "{}",
        message(std::panic::catch_unwind(|| endless_after::<HashSet<u32>>().0.len()))
    );
}
