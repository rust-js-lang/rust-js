// An adapter held in a variable and given to what steps it from its back,
// a `rev()` or a function asking `DoubleEndedIterator` (#484): made knowing
// its back where it's made, `$filterEnds($mapEnds(..))` of a `filter_map`,
// whatever its function is: a constructor, `Some`, whose kept values look
// like `None`, a closure, or a function; and a map's or a filter's, of an
// array's items or a range's. One made where it's reversed keeps its own
// form.
fn halve(x: u32) -> Option<u32> {
    if x % 2 == 0 { Some(x / 2) } else { None }
}

fn last<I: DoubleEndedIterator<Item = u32>>(mut items: I) -> Option<u32> {
    items.next_back()
}

fn main() {
    let kept = vec![Some(1), None, Some(3)].into_iter().filter_map(Some);
    println!("{:?}", kept.rev().collect::<Vec<Option<u32>>>());

    let present = vec![Some(1), None, Some(3)].into_iter().filter_map(|x| x);
    println!("{:?}", present.rev().collect::<Vec<u32>>());

    let halves = (1..=8).filter_map(halve);
    let mut back = halves.rev();
    println!("{:?} {:?}", back.next(), back.next());
    println!("{:?}", back.collect::<Vec<_>>());

    let v = vec![1, 2, 3];
    let more = v.iter().map(|x| x + 1);
    let odd = (1..6).filter(|x| x % 2 == 1);
    println!("{:?} {:?}", more.rev().collect::<Vec<u32>>(), odd.rev().collect::<Vec<u32>>());

    let doubled = v.iter().filter_map(|&x| (x > 1).then_some(x * 2));
    println!("{:?}", last(doubled));

    // Made where it's reversed, by its own form: the array's items
    // reversed, filtered.
    println!("{:?}", v.iter().filter(|x| **x != 2).rev().collect::<Vec<_>>());
}
