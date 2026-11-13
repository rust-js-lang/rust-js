// A `step_by(n)` of a reversed range a variable holds, collected into a
// `Vec` in a crate that reads its capacity: std grows it by its hint, which
// the runtime's `$lazyRangeBack` gives as std's `StepBy` does, so it needs
// no length made where it's collected. It was refused, "as an
// `ExactSizeIterator`, which rust-js can't count", as the variable shows
// nothing of what made it (#1573).
fn main() {
    for (n, step) in [(7usize, 2usize), (100, 3), (37, 2), (64, 5), (9, 1), (1000, 7), (0, 3)] {
        let it = (0..n).rev().step_by(step);
        let v = it.collect::<Vec<_>>();
        println!("{n} {step}: {} {} {:?}", v.len(), v.capacity(), v.first());
    }
}
