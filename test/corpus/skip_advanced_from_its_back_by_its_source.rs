// A `skip(n)` advanced from its back passes it on to its source's own
// `advance_back_by`, as many as it gives, as std's `Skip::advance_back_by`
// does: a `Vec`'s drops what it passes as one slice, front to back. And
// its length is std's `ExactSizeIterator::len`, of its size hint (#2112).

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
    let n = ds().into_iter().skip(1).rev().take(2).count();
    println!("skip rev take count {n}");
    let n = ds().into_iter().skip(3).rev().take(4).count();
    println!("past what it gives {n}");
    let it = ds().into_iter().skip(2);
    println!("len {}", it.len());
}
