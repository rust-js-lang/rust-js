// A lazy adapter that holds a function with a destructor, `map(f)`'s,
// `filter(p)`'s: made closing, `$closing(it, ..)`, which closes its source,
// then drops its function, as Rust drops a `Map`'s fields, when it's
// consumed, dropped or given to generic code; a generic `F` too, its caller's
// drop (ADR 0485).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn scaled(v: &[u32]) -> Vec<u32> {
    let factor = Loud(3);
    v.iter()
        .map(move |x| {
            x * factor.0 + {
                let f = &factor;
                f.0
            }
        })
        .collect()
}

fn kept<F: Fn(u32) -> u32>(f: F, v: Vec<u32>) -> Vec<u32> {
    v.into_iter().map(move |x| f(x)).collect()
}

fn count_by<I: Iterator, K, F: FnMut(I::Item) -> K>(it: I, f: F) -> usize {
    it.map(f).count()
}

fn main() {
    println!("{:?}", scaled(&[1, 2]));
    println!("after scaled");
    let shared = Loud(10);
    let total: u32 = [1u32, 2]
        .iter()
        .map(move |x| {
            let s = &shared;
            x + s.0
        })
        .sum();
    println!("total {total}");
    let base = Loud(5);
    println!(
        "{:?}",
        kept(
            move |x| {
                let b = &base;
                x + b.0
            },
            vec![1, 2]
        )
    );
    let tag = Loud(7);
    let mut filtered = [1u32, 2, 3, 4].into_iter().filter(move |x| {
        let t = &tag;
        x % 2 == t.0 % 2
    });
    println!("{:?}", filtered.next());
    drop(filtered);
    let label = Loud(9);
    println!(
        "{}",
        count_by([1u32, 2, 3].into_iter(), move |x| {
            let l = &label;
            x + l.0
        })
    );
    println!("end");
}
