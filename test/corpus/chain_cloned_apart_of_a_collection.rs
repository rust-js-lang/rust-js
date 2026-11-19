// A chain cloned apart whose other side is a collection, an array or a
// `Vec` given by value or a `VecDeque` by reference: that side is made its
// iterator where the chain is, as `into_iter()` makes it, so a clone, a
// `cycle()`'s each turn's too, clones how far it's stepped, where the
// collection itself was passed and its clone threw (#2149, ADR 0525).
use std::collections::VecDeque;

fn main() {
    let chained = [1, 2].iter().copied().chain([3]);
    println!("{:?}", chained.cycle().take(7).collect::<Vec<_>>());

    let mut turned = [1, 2].iter().copied().chain([3, 4, 5]);
    turned.next();
    turned.next();
    turned.next();
    println!("{:?}", turned.cycle().take(5).collect::<Vec<_>>());

    let mut stepped = [1, 2].iter().copied().chain([3, 4]);
    stepped.next();
    stepped.next();
    let copy = stepped.clone();
    stepped.next();
    println!("{:?} {:?}", stepped.collect::<Vec<_>>(), copy.collect::<Vec<_>>());

    let owned = vec![10, 20];
    let mut items = vec![1].into_iter().chain(owned);
    items.next();
    items.next();
    let again = items.clone();
    println!("{:?} {:?}", items.next(), again.collect::<Vec<_>>());

    let queue: VecDeque<i32> = [30, 40].into_iter().collect();
    println!("{:?}", [5].iter().chain(&queue).cycle().take(5).collect::<Vec<_>>());
}
