// `size_hint()` of `iter::once(x)` and `iter::once_with(f)` is std's,
// exact: `(1, Some(1))` until it's stepped, then `(0, Some(0))` (#1375).
// `once(x)` is `[x]`, an array of items, `$iter([x])` where its hint is
// asked, as an `Option`'s `into_iter()` is; `once_with(f)` is `$onceWith`,
// which hints as std's `OnceWith` does, by whether it holds `f`, still
// called only when it's first stepped. So a `peekable()` of either hints,
// and a `Vec` grown by a chain or a zip of them has std's capacity.
fn main() {
    let mut o = std::iter::once(1);
    println!("once {:?}", o.size_hint());
    o.next();
    println!("once stepped {:?}", o.size_hint());
    println!("once peekable {:?}", std::iter::once(1).peekable().size_hint());

    let mut w = std::iter::once_with(|| 1);
    println!("once_with {:?}", w.size_hint());
    w.next();
    println!("once_with stepped {:?}", w.size_hint());
    println!(
        "once_with peekable {:?}",
        std::iter::once_with(|| 2).peekable().size_hint()
    );

    let mut p = std::iter::once_with(|| {
        println!("called");
        5
    })
    .peekable();
    println!("held {:?}", p.size_hint());
    let peeked = p.peek().copied();
    println!("peeked {:?} {:?}", peeked, p.size_hint());

    let chained: Vec<u32> = std::iter::once_with(|| 1)
        .chain((0..5).filter(|x| x % 2 == 0))
        .collect();
    println!("chained {:?} {}", chained, chained.capacity());
    let zipped: Vec<(u32, u32)> = std::iter::once_with(|| 7).zip(std::iter::once(8)).collect();
    println!("zipped {:?} {}", zipped, zipped.capacity());
    let total: u32 = std::iter::once(3).map(|x| x * 2).sum();
    println!("total {total}");
}
