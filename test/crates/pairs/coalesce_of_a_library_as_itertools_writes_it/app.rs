//! A consumer of the library's `dedup` and `coalesce`, as itertools' probe
//! uses them, of values that drop and of numbers: the library's `next`,
//! compiled here from its MIR, with its cleanup's write (#1681).
#[derive(PartialEq)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let deduped: Vec<&u32> = dep::dedup([1, 1, 2, 2, 2, 3, 1].iter()).collect();
    println!("dedup {deduped:?}");
    let louds: Vec<u32> = dep::dedup(vec![Loud(1), Loud(1), Loud(2)].into_iter())
        .map(|l| l.0)
        .collect();
    println!("louds {louds:?}");
    let sums: Vec<i32> = dep::coalesce([1, 2, -3, 4].into_iter(), |a: i32, b: i32| {
        if (a >= 0) == (b >= 0) { Ok(a + b) } else { Err((a, b)) }
    })
    .collect();
    println!("coalesce {sums:?}");
}
