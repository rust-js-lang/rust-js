//@ library-differs
// A blanket impl's default copied into its impl, making closures that name
// their own generics past the default's, their kind, signature and
// captures: `|_| ()` and a closure capturing `compare`, as itertools'
// `minmax_by` makes them, are read in the impl's terms with their own
// generics as they are, where rust-js crashed, "type parameter out of
// range when instantiating" (#828).
use std::cmp::Ordering;

fn extremes<I, K, F, L>(it: I, mut key: F, mut less: L) -> Option<(I::Item, I::Item)>
where
    I: Iterator,
    I::Item: Clone,
    F: FnMut(&I::Item) -> K,
    L: FnMut(&I::Item, &I::Item, &K, &K) -> bool,
{
    let mut best: Option<(I::Item, I::Item)> = None;
    for x in it {
        let k = key(&x);
        best = Some(match best {
            None => (x.clone(), x),
            Some((lo, hi)) => {
                let lo = if less(&x, &lo, &k, &k) { x.clone() } else { lo };
                let hi = if less(&hi, &x, &k, &k) { x } else { hi };
                (lo, hi)
            }
        });
    }
    best
}

trait Ext: Iterator {
    fn extremes_by<F>(self, mut compare: F) -> Option<(Self::Item, Self::Item)>
    where
        Self: Sized,
        Self::Item: Clone,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        extremes(self, |_| (), |x, y, _, _| Ordering::Less == compare(x, y))
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

fn main() {
    let found = [3, 1, 4, 1, 5].into_iter().extremes_by(|a, b| a.cmp(b));
    println!("{found:?}");
}
