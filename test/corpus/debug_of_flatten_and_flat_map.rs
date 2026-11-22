//@ library-differs
// `{:?}` of std's `Flatten` and `FlatMap` is std's own `Debug`, compiled
// into `@rust-js/std`: their `inner`, std's private `FlattenCompat`, with
// its source `Fuse`d and what's taken from each end, `frontiter` and
// `backiter`, as stepping leaves them, through generic code, `{:#?}`, an
// `Option` and a struct's derived `Debug` alike (#2252).
use std::fmt::Debug;

fn show<T: Debug>(t: &T) -> String {
    format!("{t:?}")
}

/// A `FlatMap` of words by a function pointer, a struct's field.
type Words = std::iter::FlatMap<std::vec::IntoIter<&'static str>, std::str::Chars<'static>, Splitter>;
type Splitter = fn(&'static str) -> std::str::Chars<'static>;

#[derive(Debug)]
struct Holder {
    words: Words,
}

fn main() {
    let fresh = vec![vec![1], vec![2]].into_iter().flatten();
    println!("{fresh:?}");
    let mut stepped = vec![vec![1, 2], vec![3, 4]].into_iter().flatten();
    stepped.next();
    stepped.next_back();
    println!("{stepped:?}");
    println!("{stepped:#?}");
    let made = (0..3).flat_map(|n| 0..n);
    println!("{made:?}");
    let mut taken = (1..4).flat_map(|n| 0..n);
    taken.next();
    taken.next();
    println!("{} {:?}", show(&taken), Some(&taken));
    let mut chars = vec!["ab", "cd"].into_iter().flat_map(|w| w.chars());
    chars.next();
    println!("{chars:?}");
    let words: Splitter = str::chars;
    let mut holder = Holder {
        words: vec!["xy", "z"].into_iter().flat_map(words),
    };
    holder.words.next_back();
    println!("{holder:?}");
    let empty = Vec::<Vec<u8>>::new().into_iter().flatten();
    println!("{empty:?}");
}
