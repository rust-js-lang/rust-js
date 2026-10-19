// A closure its maker gives back, the type its `impl Fn` hides, holding a
// value with a destructor, carries its drop, `$dropping(f, drop)`, as JS's
// `Symbol.dispose`; dropped where its maker's variables aren't seen, it's
// `$dispose(f)`, as itertools' `key_to_cmp` gives one to `k_smallest_by`
// (ADR 0484).
use std::cmp::Ordering;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn key_to_cmp<T, K, F>(mut key: F) -> impl FnMut(&T, &T) -> Ordering
where
    F: FnMut(&T) -> K,
    K: Ord,
{
    move |a, b| key(a).cmp(&key(b))
}

fn smallest<T: Clone, F: FnMut(&T, &T) -> Ordering>(items: &[T], mut cmp: F) -> Option<T> {
    let mut best: Option<&T> = None;
    for item in items {
        if best.is_none_or(|b| cmp(item, b) == Ordering::Less) {
            best = Some(item);
        }
    }
    best.cloned()
}

fn by_len() -> impl Fn(&String) -> usize {
    let tag = Loud(7);
    move |s| {
        let _ = &tag;
        s.len()
    }
}

fn main() {
    let weights = Loud(1);
    let cmp = key_to_cmp(move |x: &u32| {
        let w = &weights;
        (*x as i64 - w.0 as i64).abs()
    });
    println!("{:?}", smallest(&[5, 2, 9], cmp));
    let f = by_len();
    println!("{}", f(&"abc".to_string()));
    drop(f);
    println!("end");
}
