// A `collect()` std makes in place, in generic code, of a chain its caller
// gives whose source `Vec` a call of the crate's hides, `opaque(v.into_iter())`:
// its capacity is that source's buffer's, which std's `SourceIter` finds
// through each adapter. The caller makes the chain on a source its adapters
// link to, so generic code's collect finds it too, where its capacity is
// read in another body (#1941). Each adapter std collects in place through,
// a chain of two, and items with a destructor.
fn opaque<I: Iterator>(it: I) -> impl Iterator<Item = I::Item> {
    it
}

fn at_0<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.collect()
}

fn louder<I: Iterator<Item = Loud>>(it: I) -> Vec<Loud> {
    it.collect()
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn kept() -> Vec<u32> {
    let mut v = Vec::with_capacity(10);
    v.extend([1, 2, 3, 4]);
    v
}

fn main() {
    let rows: Vec<(&str, Vec<u32>)> = vec![
        ("map", at_0(opaque(kept().into_iter()).map(|x| x * 2))),
        ("filter", at_0(opaque(kept().into_iter()).filter(|x| x % 2 == 0))),
        (
            "filter_map",
            at_0(opaque(kept().into_iter()).filter_map(|x| (x > 1).then_some(x))),
        ),
        ("skip", at_0(opaque(kept().into_iter()).skip(1))),
        ("take", at_0(opaque(kept().into_iter()).take(2))),
        ("inspect", at_0(opaque(kept().into_iter()).inspect(|_| ()))),
        (
            "enumerate",
            at_0(opaque(kept().into_iter()).enumerate().map(|(i, x)| x + i as u32)),
        ),
        (
            "map_filter",
            at_0(opaque(kept().into_iter()).map(|x| x + 1).filter(|x| x % 2 == 0)),
        ),
        ("take_while", at_0(opaque(kept().into_iter()).take_while(|x| *x < 3))),
        ("direct", at_0(kept().into_iter().map(|x| x + 1))),
    ];
    for (name, v) in rows {
        println!("{name} {v:?} {}", v.capacity());
    }
    let mut louds = Vec::with_capacity(6);
    louds.extend([Loud(1), Loud(2), Loud(3)]);
    let tens = louder(opaque(louds.into_iter()).map(|l| Loud(l.0 * 10)));
    println!("louds {} {}", tens.len(), tens.capacity());
}
