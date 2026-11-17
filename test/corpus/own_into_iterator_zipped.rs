// The crate's own `IntoIterator`, `&Board`'s, given to a `zip`: its own
// `into_iter` runs as the zip is made, before its first step, as std's
// `zip` calls `into_iter` (#1940); one that panics unwinds out of the
// `zip`. With std's iterator structs it's refused, as std's compiled zip
// would step it by index (#2020).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Board {
    cells: Vec<u32>,
    bad: bool,
}

impl<'a> IntoIterator for &'a Board {
    type Item = u32;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, u32>>;
    fn into_iter(self) -> Self::IntoIter {
        println!("into_iter &Board");
        assert!(!self.bad, "a bad board");
        self.cells.iter().copied()
    }
}

fn main() {
    let b = Board {
        cells: vec![1, 2, 3],
        bad: false,
    };
    let z = [7u32, 8].iter().copied().zip(&b);
    println!("zip made");
    let z: Vec<_> = z.collect();
    println!("{z:?}");
    let bad = Board {
        cells: vec![5, 6],
        bad: true,
    };
    let r = catch_unwind(AssertUnwindSafe(|| {
        let z: Vec<(u32, u32)> = [1u32].iter().copied().zip(&bad).collect();
        println!("{z:?}");
    }));
    println!("{}", r.is_err());
}
