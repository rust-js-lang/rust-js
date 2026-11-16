// A program's local named as a helper of std's that its scope reads,
// `copied_clone` beside the `copiedClone` a clone of a `copied()` given to
// generic code reads, and `chain` beside an iterator's chain: each local
// named apart from the helper, `copiedClone$1`, which is imported by its
// word, as a local is from a function of the module's (#1975).
fn show<I: Iterator + Clone>(it: I) -> usize
where
    I::Item: std::fmt::Debug,
{
    let copy = it.clone();
    println!("{:?} {:?}", copy.collect::<Vec<_>>(), it.collect::<Vec<_>>());
    3
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    let copied_clone = items.len() + 1;
    let shown = show(items.iter().map(|x| x).copied().map(|x| x * 10));
    println!("{copied_clone} {shown}");
    let chain: Vec<i32> = items.iter().copied().chain(6..8).collect();
    println!("{chain:?}");
    for map_clone in [items.len(), copied_clone] {
        let shown = show(items.iter().map(|x| x + map_clone as i32));
        println!("{map_clone} {shown}");
    }
}
