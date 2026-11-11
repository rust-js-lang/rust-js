// A trait's default copied into a blanket impl, itertools'
// `multi_cartesian_product(self)`, gives its `Self` to a function that
// collects what it makes of it into a `Vec`, which std may do in place: the
// impl that copies the default asks its own type's `IInPlace` of its
// caller, as the callee asks it of the default (#1313). A panic part-way
// drops what was collected as std's in-place collect drops it, before
// what's left of its source.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I> {
    _it: I,
    tag: Loud,
}

fn held_all<H>(iters: H) -> Vec<Held<<H::Item as IntoIterator>::IntoIter>>
where
    H: Iterator,
    H::Item: IntoIterator,
{
    let mut n = 0;
    iters
        .map(|i| {
            n += 1;
            assert!(n < 3, "at {n}");
            Held {
                _it: i.into_iter(),
                tag: Loud(n),
            }
        })
        .collect()
}

trait Tools: Iterator {
    fn held(self) -> usize
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        let held = held_all(self);
        held.iter().map(|h| h.tag.0 as usize).sum()
    }
}

impl<T: Iterator> Tools for T {}

fn main() {
    println!("{}", vec![vec![1u32, 2], vec![3]].into_iter().held());
    println!("{}", [vec![1u8]].into_iter().held());
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![vec![Loud(10)], vec![Loud(20)], vec![Loud(30)], vec![Loud(40)]]
            .into_iter()
            .held()
    }));
    println!("panicked: {}", r.is_err());
}
