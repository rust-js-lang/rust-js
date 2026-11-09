// `char::decode_utf16(units)` is std's `DecodeUtf16`: each `char` decoded
// as it's asked, one unit stepped or a pair, a unit read past an unpaired
// leading surrogate kept for the next; its `size_hint()` std's, of its
// source's; and a panic in its source's step ends it, the `char`s decoded
// before it shown (#1151).
fn main() {
    let units = [0x61u16, 0xD83D, 0xDE00, 0xD800, 0x62, 0xDC00, 0xD800];
    for r in char::decode_utf16(units.iter().map(|&u| {
        println!("step {u:x}");
        u
    })) {
        println!("{r:?}");
    }
    let mut d = char::decode_utf16(units);
    println!("{:?} {:?}", d.size_hint(), d.next());
    println!("{:?} {:?}", d.next(), d.size_hint());
    println!("{:?}", d.next());
    println!("{:?}", d.size_hint());
    println!("{:?}", d.next());
    println!("{:?} {:?}", d.next(), d.size_hint());
    let all: Vec<_> = d.collect();
    println!("{all:?}");
    let s: String = char::decode_utf16(units).map(|r| r.unwrap_or('?')).collect();
    println!("{s}");
    let r = std::panic::catch_unwind(|| {
        let n = char::decode_utf16([1u16, 2, 3].into_iter().map(|d| {
            println!("unit {d}");
            if d == 2 { panic!("at 2") } else { 0x61 + d }
        }))
        .inspect(|r| println!("{r:?}"))
        .count();
        println!("{n}");
    });
    println!("{}", r.is_err());
}
