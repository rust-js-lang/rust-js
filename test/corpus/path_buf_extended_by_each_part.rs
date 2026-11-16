// A `PathBuf`'s `extend`, std's `push` of each part's `as_ref()` as it's
// stepped (#1640): `OsString`s, `&str`s, an absolute part that replaces
// what came before, `&Path`s, a crate type's `AsRef<Path>` and a path's
// `components()`. Each part taken by value is dropped after its push, as
// the closure's `p` is, a panicking `as_ref()`'s too, and what's left after
// it. What's pushed before a panic part-way stays, as the
// `PathBuf` is changed in place; no hint is asked, as std's `for_each` asks
// none.
use std::ffi::OsString;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

struct Dir(&'static str);

impl AsRef<Path> for Dir {
    fn as_ref(&self) -> &Path {
        if self.0 == "bad" {
            panic!("no path of bad");
        }
        Path::new(self.0)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        println!("drop Dir {}", self.0);
    }
}

/// Parts that say when their hint is asked, which std's `PathBuf::extend`
/// never does: it pushes each as `for_each` steps it.
struct Parts(Vec<&'static str>);

impl Iterator for Parts {
    type Item = &'static str;

    fn next(&mut self) -> Option<&'static str> {
        if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint asked");
        (self.0.len(), Some(self.0.len()))
    }
}

fn main() {
    let mut p = PathBuf::from("a");
    let parts = vec![OsString::from("b"), OsString::from("c")];
    p.extend(parts.iter());
    println!("{}", p.display());

    let mut q = PathBuf::new();
    q.extend(["x", "/abs", "y"]);
    println!("{}", q.display());

    let mut u = PathBuf::from("base");
    u.extend(vec![Path::new("p"), Path::new("q/")]);
    println!("{:?}", u);

    let mut d = PathBuf::from("root");
    d.extend([Dir("one"), Dir("two")]);
    println!("{}", d.display());

    let mut c = PathBuf::from("/srv");
    c.extend(Path::new("data/./logs").components());
    println!("{}", c.display());

    let mut partial = PathBuf::from("start");
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        partial.extend(["first", "second", "third"].iter().map(|s| {
            if *s == "third" {
                panic!("stopped at {s}");
            }
            *s
        }));
    }));
    println!("{} {}", result.is_err(), partial.display());

    let mut stopped = PathBuf::from("dirs");
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        stopped.extend([Dir("x"), Dir("bad"), Dir("y")]);
    }));
    println!("{} {}", result.is_err(), stopped.display());

    let mut owned = PathBuf::from("owned");
    owned.extend(vec![Dir("v1"), Dir("v2")]);
    owned.extend(Some(Dir("some")));
    println!("{}", owned.display());

    let mut hinted = PathBuf::from("h");
    hinted.extend(Parts(vec!["i", "j"]));
    println!("{}", hinted.display());

    let mut empty = PathBuf::from("same");
    empty.extend(Vec::<String>::new());
    println!("{}", empty.display());
}
