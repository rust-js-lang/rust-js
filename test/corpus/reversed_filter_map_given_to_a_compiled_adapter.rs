// The `rev()` of a `filter_map` a variable holds, given to std's compiled
// adapter, a `fuse()` or a `step_by(1)`: std's compiled `Rev`, which steps
// what it holds by its type's `next_back`, so that type is made knowing its
// back wherever it's made, whatever its function is, a constructor, `Some`,
// a closure or a function, where it was refused ("the `DoubleEndedIterator`
// of a `FilterMap`").
fn same(x: Option<u32>) -> Option<Option<u32>> {
    Some(x)
}

fn main() {
    let kept = vec![Some(1), None, Some(3)].into_iter().filter_map(Some);
    println!("{:?}", kept.rev().fuse().collect::<Vec<Option<u32>>>());
    let by_closure = vec![Some(1), None, Some(3)]
        .into_iter()
        .filter_map(|x| x.map(|n| n * 2));
    println!("{:?}", by_closure.rev().step_by(1).collect::<Vec<u32>>());
    let by_function = vec![Some(1), None, Some(3)].into_iter().filter_map(same);
    println!(
        "{:?}",
        by_function.rev().fuse().collect::<Vec<Option<u32>>>()
    );
}
