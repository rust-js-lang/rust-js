// `{:?}` of std's enums whose variants hold what they hold by place, and
// whose `Debug` is derived: `Bound`'s `Included(3)` and `Unbounded`,
// `ControlFlow`'s `Break(..)`, each as a derived `Debug` shows it, `{:#?}`
// too (ADR 0033).
use std::ops::{Bound, ControlFlow};

fn main() {
    let bounds: [Bound<u32>; 3] = [Bound::Included(3), Bound::Excluded(9), Bound::Unbounded];
    println!("{:?}", bounds);
    let flow: ControlFlow<&str, u32> = ControlFlow::Break("stop");
    let more: ControlFlow<&str, u32> = ControlFlow::Continue(2);
    println!("{flow:?} {more:?}");
    println!("{:#?}", Bound::Included((1u8, 'x')));
    let pair: (Bound<&str>, Bound<&str>) = (Bound::Excluded("a"), Bound::Unbounded);
    println!("{pair:?}");
}
