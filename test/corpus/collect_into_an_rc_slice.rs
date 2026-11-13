// `collect()` into an `Rc<[T]>` or an `Arc<[T]>`, directly or through
// generic code's `FromIterator` dictionary (#1678): std collects a `Vec`
// and makes the `Rc` of it, so its items are stepped as a `Vec`'s are,
// and an `Rc` whose counts are read is counted.
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;

fn collected<C: FromIterator<u32>>(v: Vec<u32>) -> C {
    v.into_iter().collect()
}

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let direct: Rc<[u32]> = vec![4, 5].into_iter().map(|x| x * 2).collect();
    println!("{direct:?} {}", direct.len());
    let generic: Rc<[u32]> = collected(vec![1, 2, 3]);
    println!("{generic:?}");
    let shared: Arc<[u32]> = collected(vec![6]);
    println!("{shared:?}");
    let empty: Rc<[String]> = Vec::<String>::new().into_iter().collect();
    println!("{empty:?} {}", empty.is_empty());

    // Counted: its strong count is read.
    let words: Rc<[&str]> = "a b c".split(' ').collect();
    let other = Rc::clone(&words);
    println!("{:?} {} {}", other, Rc::strong_count(&words), words[1]);

    // A panic part-way through what isn't `TrustedLen` drops what was
    // collected so far, as a `Vec`'s does.
    let stopped = catch_unwind(AssertUnwindSafe(|| {
        let made: Rc<[Loud]> = (1..5)
            .filter(|i| i % 7 != 0)
            .map(|i| {
                if i == 3 {
                    panic!("at {i}");
                }
                Loud(i)
            })
            .collect();
        made.len()
    }));
    println!("stopped {}", stopped.is_err());
}
