//@ library-differs
// An unstable sort, or `select_nth_unstable`, of a type parameter's items:
// std's algorithm, as its caller's type picks it by its layout, `TSort`,
// which leaves equal items where std's does (ADR 0471).
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy)]
struct Tagged(u8, char);

fn by_key(a: &Tagged, b: &Tagged) -> Ordering {
    a.0.cmp(&b.0)
}

fn sorted<T, F: FnMut(&T, &T) -> Ordering>(mut items: Vec<T>, compare: F) -> Vec<T> {
    items.sort_unstable_by(compare);
    items
}

fn smallest<T: Ord + Clone>(mut items: Vec<T>, k: usize) -> Vec<T> {
    items.select_nth_unstable(k);
    items[..k].to_vec()
}

trait Sorted: Iterator {
    fn sorted_unstable(self) -> Vec<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.sort_unstable();
        v
    }
}

impl<I: Iterator> Sorted for I {}

fn main() {
    let tags: Vec<Tagged> = (0..40).map(|i| Tagged((i * 7 % 5) as u8, (b'a' + (i % 26) as u8) as char)).collect();
    let by = sorted(tags.clone(), by_key);
    println!("{:?}", by.iter().map(|t| t.1).collect::<String>());
    let wide: Vec<(u64, u64, u64, char)> = (0..30).map(|i| ((i * 3 % 4) as u64, 0, 0, (b'a' + i as u8) as char)).collect();
    let wide = sorted(wide, |a, b| a.0.cmp(&b.0));
    println!("{:?}", wide.iter().map(|t| t.3).collect::<String>());
    println!("{:?}", smallest(vec![9, 3, 7, 1, 8, 2], 3));
    println!("{:?}", vec![5, 3, 9, 1].into_iter().sorted_unstable());
}
