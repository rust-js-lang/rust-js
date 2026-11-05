// `Vec::from_iter(iter)` of a type parameter's iterator, as itertools'
// `sorted_by` writes it, asks its caller what `collect()` asks: what std
// may collect it in, in place, and its items' layout, by which a panic
// drops what it collected or what's left first (#672, #737).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop 3");
        }
    }
}

fn all<I: Iterator>(iter: I) -> Vec<I::Item> {
    Vec::from_iter(iter)
}

fn sorted<I: Iterator>(iter: I) -> Vec<I::Item>
where
    I::Item: Ord,
{
    let mut v = Vec::from_iter(iter);
    v.sort();
    v
}

fn main() {
    let v = all(vec![D(1), D(2)].into_iter().map(|d| D(d.0 + 10)));
    println!("{:?}", v.iter().map(|d| d.0).collect::<Vec<_>>());
    drop(v);
    println!("{:?}", sorted(vec![3, 1, 2].into_iter()));
    // Of one whose map drops what it's given, a panic: what it collected
    // and what's left dropped, as std's in-place collect drops them.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v = all(vec![D(1), D(3), D(5)].into_iter().map(|d| D(d.0 + 10)));
        v.len()
    }));
    println!("panicked {}", r.is_err());
}
