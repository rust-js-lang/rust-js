// A `flatten()` and a `flat_map()` of inner iterators that aren't arrays
// hint as std's `FlattenCompat` does: what's left of the inner ones it's
// in, front and back, its lower bound; an upper bound only where its
// source is done. A `Vec` it grows, in a crate that reads a capacity,
// grows by that lower bound, as std's (#1892).
fn main() {
    let words = ["ab", "", "cde", "f"];
    let mut flat = words.iter().map(|w| w.as_bytes()).flatten();
    println!("{:?}", flat.size_hint());
    flat.next();
    println!("{:?}", flat.size_hint());
    flat.next_back();
    println!("{:?}", flat.size_hint());
    let rest: Vec<u8> = flat.copied().collect();
    println!("{:?} {}", rest, rest.capacity());

    let mut mapped = words.iter().flat_map(|w| w.as_bytes().iter().map(|b| b + 1));
    println!("{:?}", mapped.size_hint());
    mapped.next();
    mapped.next_back();
    println!("{:?}", mapped.size_hint());
    let rest: Vec<u8> = mapped.collect();
    println!("{:?} {}", rest, rest.capacity());

    let mut back = words.iter().map(|w| w.as_bytes()).flatten().rev();
    back.next();
    println!("{:?}", back.size_hint());
    let rest: Vec<u8> = back.copied().collect();
    println!("{:?} {}", rest, rest.capacity());

    let mut fm = words.iter().flat_map(|w| w.as_bytes().iter()).rev();
    fm.next();
    fm.next();
    println!("{:?}", fm.size_hint());
    let rest: Vec<u8> = fm.copied().collect();
    println!("{:?} {}", rest, rest.capacity());

    let mut drained = words.iter().map(|w| w.as_bytes()).flatten();
    for _ in 0..5 {
        drained.next();
    }
    println!("{:?}", drained.size_hint());
    let mut grown: Vec<u8> = Vec::new();
    grown.extend(words.iter().map(|w| w.as_bytes()).flatten().copied());
    println!("{} {}", grown.len(), grown.capacity());
}
