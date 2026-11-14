// A `Cow` given to generic code bound by `AsRef` or `Borrow` of what it
// borrows or owns: std's impls, each what its `Deref` is, the text or the
// items, borrowed or owned alike, and a `Cow<OsStr>`'s `AsRef<Path>`,
// its text (#1481).

use std::borrow::{Borrow, Cow};
use std::collections::HashMap;

fn upper<T: AsRef<str>>(text: T) -> String {
    text.as_ref().to_uppercase()
}

fn total<T: AsRef<[u32]>>(items: T) -> u32 {
    items.as_ref().iter().sum()
}

fn borrowed_len<T: Borrow<str>>(text: T) -> usize {
    text.borrow().len()
}

fn lookup<K: Borrow<str>>(map: &HashMap<String, u32>, key: K) -> Option<u32> {
    map.get(key.borrow()).copied()
}

fn shown<P: AsRef<std::path::Path>>(path: P) -> String {
    path.as_ref().display().to_string()
}

fn trimmed(text: &str) -> Cow<'_, str> {
    if text.starts_with(' ') || text.ends_with(' ') {
        Cow::Owned(text.trim().to_owned())
    } else {
        Cow::Borrowed(text)
    }
}

fn main() {
    println!("{}", upper(Cow::Borrowed("cow")));
    println!("{}", upper(Cow::<str>::Owned(String::from("owned"))));
    println!("{}", upper(trimmed("  padded ")));
    println!("{}", upper(&trimmed("plain")));
    let items: Cow<[u32]> = Cow::Borrowed(&[1, 2, 3]);
    let owned: Cow<[u32]> = Cow::Owned(vec![4, 5]);
    println!("{} {}", total(items.clone()), total(&owned));
    println!(
        "{} {}",
        borrowed_len(trimmed(" ab ")),
        borrowed_len(Cow::Borrowed("abc"))
    );
    let mut map = HashMap::new();
    map.insert(String::from("k"), 7);
    println!(
        "{:?} {:?}",
        lookup(&map, trimmed(" k")),
        lookup(&map, Cow::Borrowed("x"))
    );
    let mut texts: Vec<Cow<str>> = vec![trimmed("b"), trimmed(" a ")];
    texts.sort();
    println!("{:?}", texts.iter().map(upper).collect::<Vec<_>>());
    println!("{items:?} {owned:?}");
    let path: Cow<std::path::Path> = Cow::Borrowed(std::path::Path::new("a/b.txt"));
    let built: Cow<std::path::Path> = Cow::Owned(std::path::PathBuf::from("c").join("d"));
    println!("{} {}", shown(path), shown(&built));
    // std's `AsRef<Path>` of a `Cow<OsStr>`, its text as a path.
    let name: Cow<std::ffi::OsStr> = Cow::Borrowed(std::ffi::OsStr::new("e.txt"));
    println!("{}", shown(name));
}
