// A fold of a `take` made `$taking`, its hint asked elsewhere in the crate,
// whose source's rest, dropped once the fold has its result, panics: std's
// fold drops `self` after it has what it folded, which the panic leaks,
// never drops; the JS stepped its last step as JS's protocol does, which
// closes it at its end, and dropped the total as a step's panic would
// (#1330).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 4 {
            panic!("at 4");
        }
    }
}

struct Total(u32);

impl Drop for Total {
    fn drop(&mut self) {
        println!("drop total {}", self.0);
    }
}

impl std::iter::Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Total {
        iter.fold(Total(0), |mut t, d| {
            t.0 += d.0;
            t
        })
    }
}

fn hint_of<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn main() {
    let r = catch_unwind(|| {
        let s: Total = ds().into_iter().take(3).sum();
        println!("sum {}", s.0);
    });
    println!("{}", r.is_err());
    println!("given hint {:?}", hint_of(vec![D(5)].into_iter().take(1)));
}
