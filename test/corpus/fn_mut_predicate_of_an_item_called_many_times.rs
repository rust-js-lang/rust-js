// A closure called by `&mut`, holding a value with a destructor, given as an
// `F: Pred<I::Item>` whose impl is for every `FnMut(&T, &T) -> bool`, and
// called once for each pair, as itertools' `kmerge_by` is given one: the
// bound names an associated type of the iterator, which isn't a function
// called by `FnOnce`, so it's no `FnOnce` call's, and what it took is
// dropped where the function that took it drops it (#2031).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Pred<T> {
    fn test(&mut self, a: &T, b: &T) -> bool;
}

impl<T, F: FnMut(&T, &T) -> bool> Pred<T> for F {
    fn test(&mut self, a: &T, b: &T) -> bool {
        self(a, b)
    }
}

fn ordered_pairs<I: Iterator, F: Pred<I::Item>>(it: I, mut f: F) -> usize {
    let items: Vec<I::Item> = it.collect();
    let mut n = 0;
    for pair in items.windows(2) {
        if f.test(&pair[0], &pair[1]) {
            n += 1;
        }
    }
    println!("compared {}", items.len().saturating_sub(1));
    n
}

fn main() {
    let d = Loud("predicate");
    let mut calls = 0;
    let less = move |a: &u32, b: &u32| {
        let held = &d;
        calls += 1;
        println!("{} call {calls}", held.0);
        a < b
    };
    println!("{}", ordered_pairs([3u32, 1, 4, 1, 5].into_iter(), less));
    println!("end");
}
