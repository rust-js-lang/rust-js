// `collect()` into an `Option` whose `Some` could look like `None`: an
// `Option<()>`, an `Option<Option<()>>` and an `Option<V>` of a generic
// `V`, each `Some` of what's collected boxed, as any such `Some` is,
// where it read as `None` (#984).
use std::collections::HashSet;

fn all_done(steps: &[Option<()>]) -> Option<()> {
    steps.iter().copied().collect()
}

fn gathered<T: Copy, V: FromIterator<T>>(items: &[Option<T>]) -> Option<V> {
    items.iter().copied().collect()
}

fn main() {
    println!("{:?}", all_done(&[Some(()), Some(())]));
    println!("{:?}", all_done(&[Some(()), None]));
    println!("{:?}", all_done(&[]));
    let mut ran = 0;
    let checked: Option<()> = (1..=3)
        .map(|n| {
            ran += n;
            (n < 5).then_some(())
        })
        .collect();
    println!("{checked:?} {ran}");
    let nested: Option<Option<()>> = [Some(Some(())), Some(None)].into_iter().collect();
    println!("{nested:?}");
    let whole: Option<Option<()>> = [Some(Some(())), Some(Some(()))].into_iter().collect();
    println!("{whole:?}");
    let unit: Option<()> = gathered(&[Some(()), Some(())]);
    println!("{unit:?}");
    let set: Option<HashSet<u8>> = gathered(&[Some(4), Some(4)]);
    println!("{:?}", set.map(|set| set.len()));
    let none: Option<()> = gathered(&[Some(()), None]);
    println!("{none:?}");
}
