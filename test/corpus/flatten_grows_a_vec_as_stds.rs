//@ library-differs
// A `rev()` of a `flatten()`, made as the runtime's `$flattenEnds`, of
// inner iterators that aren't arrays, grows a `Vec` as std's does in a
// crate that reads a capacity: by std's `FlattenCompat::size_hint`'s lower
// bound, what's left of the inner ones it's in, front and back, which
// `$flattenEnds` hints as std's (`$flatHint`, #1892). Beside a trait
// default's generic `flat_map`, as #1892's neighbour pair was.
pub trait Ext: Iterator {
    fn flat(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).collect()
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

fn main() {
    let vecs: Vec<u32> = vec![vec![1u32, 2, 3]].into_iter().flat();
    println!("{} {}", vecs.len(), vecs.capacity());
    let words = ["ab", "", "cde", "fghij"];
    let v: Vec<u8> = words.iter().map(|w| w.as_bytes()).flatten().rev().copied().collect();
    println!("{:?} {}", v, v.capacity());
    let mut back = words.iter().map(|w| w.as_bytes()).flatten().rev();
    back.next();
    back.next_back();
    let w: Vec<u8> = back.copied().collect();
    println!("{:?} {}", w, w.capacity());
}
