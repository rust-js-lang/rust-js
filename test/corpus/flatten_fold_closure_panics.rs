// A `flat_map()` folded from either end whose closure panics as it makes an
// inner: the accumulator the fold holds is dropped as the panic unwinds,
// then what's left, as std's `FlattenCompat` folds stage it (#744).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Acc(u32);

impl Drop for Acc {
    fn drop(&mut self) {
        println!("drop acc {}", self.0);
    }
}

fn add(mut a: Acc, d: D) -> Acc {
    a.0 += d.0;
    a
}

fn main() {
    let mut calls = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1), D(2), D(3)]
            .into_iter()
            .flat_map(|x| {
                calls += 1;
                assert!(calls != 2, "flat_map");
                vec![D(x.0 + 10), x]
            })
            .rfold(Acc(0), add)
            .0
    }));
    println!("rfold ok {}", r.is_ok());
    let mut calls = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1), D(2), D(3)]
            .into_iter()
            .flat_map(|x| {
                calls += 1;
                assert!(calls != 2, "flat_map");
                vec![D(x.0 + 10), x]
            })
            .fold(Acc(0), add)
            .0
    }));
    println!("fold ok {}", r.is_ok());
    let mut calls = 0;
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut it = vec![D(1), D(2), D(3)].into_iter().flat_map(|x| {
            calls += 1;
            assert!(calls != 3, "flat_map");
            vec![D(x.0 + 10), x]
        });
        let back = it.next_back().map(|d| d.0);
        (back, it.rfold(Acc(0), add).0)
    }));
    println!("after next_back ok {}", r.is_ok());
}
