// A flatten asks each inner iterator what std's `FlattenCompat` asks it:
// its `last` each inner's own `last`, by its source's fold; its `nth` and
// `advance_by` each inner's own `advance_by`, a `Vec`'s, which drops what
// it passes as one slice, front to back; and from its back, each inner's
// own `advance_back_by`, so `rev().take(2).count()` drops `11, 12` where
// stepping each would drop `12, 11` (#2123).

struct Loud(Vec<u32>);

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
    }
    fn last(self) -> Option<u32> {
        println!("own last");
        self.0.last().copied()
    }
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds(from: u32) -> Vec<D> {
    (from..from + 3).map(D).collect()
}

fn main() {
    let last = vec![Loud(vec![1, 2]), Loud(vec![3])].into_iter().flatten().last();
    println!("flatten last {last:?}");
    let n = vec![ds(1), ds(10)].into_iter().flatten().take(2).count();
    println!("flatten take count {n}");
    let x = vec![ds(1), ds(10)].into_iter().flatten().nth(4).map(|d| d.0);
    println!("flatten nth {x:?}");
    let n = vec![ds(1), ds(10)].into_iter().flatten().rev().take(2).count();
    println!("flatten rev take count {n}");
    let x = vec![ds(1), ds(10)].into_iter().flatten().rev().nth(4).map(|d| d.0);
    println!("flatten rev nth {x:?}");
    let n = vec![vec![1u8, 2], vec![3]].into_iter().flat_map(|v| v.into_iter()).skip(2).count();
    println!("flat_map skip count {n}");
}
