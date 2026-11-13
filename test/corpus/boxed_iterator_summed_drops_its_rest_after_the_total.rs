use std::iter::Sum;
use std::panic::{self, AssertUnwindSafe};

struct D(u32);
impl Drop for D { fn drop(&mut self) { println!("drop {}", self.0); } }
struct Total(u32);
impl Drop for Total { fn drop(&mut self) { println!("drop total {}", self.0); } }
impl Sum<D> for Total {
    fn sum<I: Iterator<Item = D>>(iter: I) -> Total {
        iter.fold(Total(0), |mut t, d| { t.0 += d.0; t })
    }
}
fn boxed(items: Vec<D>) -> Box<dyn Iterator<Item = D>> { Box::new(items.into_iter()) }

fn main() {
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut calls = 0;
        let total: Total = boxed(vec![D(1), D(2), D(3)])
            .map(|x| { calls += 1; if calls == 2 { panic!("map"); } D(x.0 + 100) })
            .sum();
        println!("total {}", total.0);
    }));
    println!("caught {}", caught.is_err());
}
