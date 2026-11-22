// The hint of a `skip_while`, `take_while`, `map_while`, `scan` and
// `filter_map` made in place over a slice's items, and of an `enumerate()`
// of each, fresh, part-consumed and exhausted, over an infinite source,
// over a range and of a type parameter's items: each made as its lazy form,
// `$lazySkipWhile` and the rest, whose hint is std's, `(0, upper)` of its
// source's. An `enumerate()` of one was refused, its JS a `map` of its
// items with their index whose source the hint's check didn't read
// (#2281).
fn points<I: Iterator<Item = u8>>(label: &str, mut it: I) {
    let fresh = it.size_hint();
    let first = it.next();
    let part = it.size_hint();
    while it.next().is_some() {}
    println!("{label}: {fresh:?} {first:?} {part:?} {:?}", it.size_hint());
}
fn generic<T: Copy + PartialOrd + Default + Into<u8>>(v: &[T]) {
    let z = T::default();
    points("g skip_while", v.iter().skip_while(|x| **x <= z).map(|x| (*x).into()));
    points("g take_while", v.iter().take_while(|x| **x >= z).map(|x| (*x).into()));
    points(
        "g enum skip_while",
        v.iter()
            .skip_while(|x| **x <= z)
            .enumerate()
            .map(|(i, x)| i as u8 + (*x).into()),
    );
    println!(
        "g enum scan {:?}",
        v.iter()
            .scan(0u8, |s, x| {
                *s += (*x).into();
                Some(*s)
            })
            .enumerate()
            .size_hint()
    );
}
pub fn main() {
    let v = [0u8, 0, 3, 1, 2, 5];
    points("skip_while", v.iter().skip_while(|x| **x < 1).copied());
    points("take_while", v.iter().take_while(|x| **x < 3).copied());
    points(
        "filter_map",
        v.iter().filter_map(|x| if *x > 1 { Some(*x) } else { None }),
    );
    points(
        "map_while",
        v.iter().map_while(|x| if *x < 3 { Some(*x) } else { None }),
    );
    points(
        "scan",
        v.iter().scan(0u8, |s, x| {
            *s += x;
            if *s < 9 { Some(*s) } else { None }
        }),
    );
    points(
        "enum skip_while",
        v.iter().skip_while(|x| **x < 1).enumerate().map(|(i, x)| i as u8 + x),
    );
    points(
        "enum take_while",
        v.iter().take_while(|x| **x < 3).enumerate().map(|(i, x)| i as u8 + x),
    );
    points(
        "enum filter_map",
        v.iter().filter_map(|x| Some(*x)).enumerate().map(|(i, x)| i as u8 + x),
    );
    points(
        "enum map_while",
        v.iter()
            .map_while(|x| if *x < 3 { Some(*x) } else { None })
            .enumerate()
            .map(|(i, x)| i as u8 + x),
    );
    points(
        "enum scan",
        v.iter()
            .scan(0u8, |s, x| {
                *s += x;
                Some(*s)
            })
            .enumerate()
            .map(|(i, x)| i as u8 + x),
    );
    let mut inf = std::iter::repeat(1u8)
        .skip_while(|x| *x == 0)
        .take_while(|x| *x == 1)
        .enumerate();
    println!("infinite {:?}", inf.size_hint());
    inf.next();
    println!("infinite part {:?}", inf.size_hint());
    println!(
        "infinite scan {:?}",
        (0u8..)
            .scan(0u8, |s, x| {
                *s = s.wrapping_add(x);
                Some(*s)
            })
            .enumerate()
            .size_hint()
    );
    generic(&[0u8, 1, 2, 0]);
    ranged();
}
fn ranged() {
    println!(
        "range scan {:?}",
        (0u8..9)
            .scan(0u8, |s, x| {
                *s = s.wrapping_add(x);
                Some(*s)
            })
            .enumerate()
            .size_hint()
    );
    println!(
        "range skip_while {:?}",
        (0u8..9).skip_while(|x| *x < 3).enumerate().size_hint()
    );
}
