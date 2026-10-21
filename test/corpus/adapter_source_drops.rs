// An adapter of an iterator of the crate's that has a destructor, stepped
// to its end by a consumer, drops its source when the chain is done, as
// Rust's does: `sum()`, `collect()`, `count()` and a `for` of a `map`.
struct Counter {
    n: u32,
    name: &'static str,
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            self.n
        })
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

fn main() {
    let total: u32 = Counter { n: 3, name: "sum" }.map(|x| x * 2).sum();
    println!("total {total}");
    let all: Vec<u32> = Counter { n: 2, name: "collect" }.filter(|x| x % 2 == 0).collect();
    println!("all {all:?}");
    let count = Counter { n: 4, name: "count" }.map(|x| x + 1).count();
    println!("count {count}");
    let tens = Counter { n: 2, name: "for" }.map(|x| x * 10);
    for x in tens {
        println!("for {x}");
    }
    println!("end");
}
