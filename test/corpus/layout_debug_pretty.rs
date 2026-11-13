// `{:#?}` of a `Layout`, nested as its derived `Debug` is: on its own, as
// a derived struct's field and in an `Option`; `{:?}` of it one line, and a
// `LayoutError`'s its name; given options, its `size` by them (ADR 0417,
// #1721, #1728).
use std::alloc::Layout;

#[derive(Debug)]
struct Sized2 {
    layout: Layout,
    count: usize,
}

fn main() {
    let layout = Layout::new::<u32>();
    println!("{layout:#?}");
    println!("{layout:?}");
    let s = Sized2 { layout, count: 2 };
    println!("{s:#?}");
    println!("{s:?}");
    println!("{:#?}", Some(Layout::new::<[u64; 3]>()));
    println!("{:#?}", Layout::from_size_align(3, 3));
    // Given options, its `size` by them, its `align` not (#1728).
    println!("{:5?}|{:<3?}|{:05?}", s, s, Layout::new::<u8>());
    println!("{:#7?}", s);
    let wide = Layout::new::<[u8; 30]>();
    println!("{:+?} {:x?} {:#X?} {:>4?}", wide, wide, wide, Layout::from_size_align(3, 3));
}
