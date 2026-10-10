// McIlroy's adversary, which decides each comparison as it's asked, so that
// a quicksort's pivots are always the worst: std's sort then falls back on
// heapsort, and `select_nth_unstable` on the median of medians. Each asks
// what std's asks, in its order, so ends with std's order.
use std::cell::{Cell, RefCell};

fn adversary(n: usize, select: Option<usize>) -> (Vec<usize>, u32) {
    let gas = n as u32;
    // Two above the rest, out of order, so it isn't one run std just reverses.
    let mut start = vec![gas; n];
    start[0] = gas + 2;
    start[1] = gas + 1;
    let values = RefCell::new(start);
    let solid = Cell::new(0u32);
    let candidate = Cell::new(0usize);
    let asked = Cell::new(0u32);
    let compare = |x: &usize, y: &usize| {
        asked.set(asked.get() + 1);
        let mut v = values.borrow_mut();
        if v[*x] == gas && v[*y] == gas {
            if *x == candidate.get() {
                v[*x] = solid.get();
            } else {
                v[*y] = solid.get();
            }
            solid.set(solid.get() + 1);
        }
        if v[*x] == gas {
            candidate.set(*x);
        } else if v[*y] == gas {
            candidate.set(*y);
        }
        v[*x].cmp(&v[*y])
    };
    let mut items: Vec<usize> = (0..n).collect();
    match select {
        Some(index) => {
            items.select_nth_unstable_by(index, compare);
        }
        None => items.sort_unstable_by(compare),
    }
    (items, asked.get())
}

fn main() {
    for n in [64, 300, 1000] {
        let (sorted, asked) = adversary(n, None);
        println!("{n} sorted after {asked}: {:?}", &sorted[..12]);
        let (picked, asked) = adversary(n, Some(n / 3));
        println!("{n} picked after {asked}: {:?}", &picked[..12]);
    }
}
