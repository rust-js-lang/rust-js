// An `enumerate()` advanced from its back passes it on to its source's
// own `advance_back_by`, as std's `Enumerate::advance_back_by` does: a
// `Vec`'s drops what it passes as one slice, front to back. A `rev()`'s
// `advance_by` asks it, as a `take(n)`'s `count` and a `skip(n)` of it do
// (#2120).

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn main() {
    let n = ds().into_iter().enumerate().rev().take(2).count();
    println!("rev take count {n}");
    let first = ds().into_iter().enumerate().rev().nth(2).map(|(i, d)| (i, d.0));
    println!("rev nth {first:?}");
    let left: Vec<usize> = ds().into_iter().enumerate().rev().skip(3).map(|(i, _)| i).collect();
    println!("rev skip {left:?}");
}
