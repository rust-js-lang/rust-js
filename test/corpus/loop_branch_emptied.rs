// A branch tidy empties inside a loop, of an `if let` whose binding is
// only read or whose body makes what nothing reads: gone in the stage
// that emptied it, `names`' or `written`'s, its test kept where it does
// anything (#247).
fn probe(n: u8) -> u8 {
    println!("probe {n}");
    n
}

fn main() {
    let len = probe(3);
    for i in 0..probe(2) {
        if let Some(product) = len.checked_mul(probe(i)) {
            let _scratch: Vec<i64> = Vec::new();
            let _ = product;
        }
    }
    let values = [probe(5), probe(9)];
    let mut best = None;
    for _ in 0..probe(2) {
        best = values.iter().copied().max();
    }
    for _ in 0..probe(1) {
        if let Some(found) = best {
            let _ = found;
        }
    }
    println!("{best:?}");
    scaled_or_last();
}

fn same<T>(x: T) -> T {
    x
}

// Of `chains`' stage, an `if let` it empties, in each branch of an `if`.
fn scaled_or_last() {
    if same(1.0f32) <= same(0.0f32) || same(3.0f64).is_infinite() {
        let factor = if same(false) { same(0u16) } else { same(65534u16) };
        if let Some(scaled) = None::<u16>.map(|e| e.wrapping_mul(factor)) {
            let _ = scaled;
        }
    } else {
        for _ in 0..same(4u32) {}
        if let Some(last) = [same(-2i64), same(5)]
            .last()
            .copied()
            .map(|e| e.wrapping_mul(same(7i64)))
        {
            let _ = last;
        }
    }
    for _ in 0..same(3u32) {}
    println!("scaled or last");
}
