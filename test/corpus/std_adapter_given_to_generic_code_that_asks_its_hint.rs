// A std adapter a caller makes and gives generic code that asks its hint,
// directly or through another generic function: made in rust-js's form,
// which knows std's hint, `$filterEnds($iter(v), p)`, `$taking($iter(v), 2)`,
// not JS's own `values().filter(p)`, which knows none (#1236).
fn hint<I: Iterator>(i: I) -> (usize, Option<usize>) {
    i.size_hint()
}

fn outer<I: Iterator<Item = u32>>(i: I) -> (usize, Option<usize>) {
    hint(i)
}

fn main() {
    let v = vec![1u32, 0, 2, 5];
    let w = [7u8, 8];
    println!(
        "{:?} {:?}",
        hint(v.iter().filter(|x| **x > 0)),
        hint(v.iter().map(|x| x + 1))
    );
    println!("{:?} {:?}", hint(v.iter().take(2)), hint(v.iter().skip(1)));
    println!("{:?} {:?}", hint(v.iter().step_by(2)), hint((0..).zip(w.iter())));
    println!(
        "{:?} {:?}",
        hint(v.iter().enumerate()),
        hint(v.iter().chain(w.iter().map(|_| &0)))
    );
    println!(
        "{:?} {:?}",
        hint(v.iter().filter_map(|x| x.checked_sub(1))),
        hint(v.iter().take_while(|x| **x > 0))
    );
    println!(
        "{:?} {:?}",
        outer(v.clone().into_iter().filter(|x| *x > 1)),
        outer(v.iter().copied().take(3))
    );
    println!("{:?} {:?}", hint(0..10u32), hint((0..10u32).step_by(3).rev()));
    let mut kept = v.iter().filter(|x| **x > 0);
    kept.next();
    println!("{:?} {:?}", hint(kept), outer([1u32, 2].into_iter().map(|x| x * 2)));
}
