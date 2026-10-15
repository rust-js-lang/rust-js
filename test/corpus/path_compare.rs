// A path compares and orders by its components, as Rust's `Path` does:
// `a//b/./c/` is `a/b/c`, `./a` isn't `a`, and `a/b` is before `a.b`, which
// as text it's after (ADR 0401). An `OsStr` beside one compares as a path;
// two compare as text.
use std::path::{Path, PathBuf};
fn main() {
    let a = Path::new("a//b/./c/");
    let b = PathBuf::from("a/b/c");
    println!("{} {} {}", a == b.as_path(), b == *a, a == Path::new("a/b/d"));
    println!("{} {}", Path::new("./a") == Path::new("a"), Path::new("/a") == Path::new("a"));
    println!("{} {}", Path::new(".") == Path::new("./"), Path::new("") == Path::new("."));
    println!("{} {}", Path::new("a//b").as_os_str() == Path::new("a/b"), Path::new("a//b").as_os_str() == Path::new("a/b").as_os_str());
    println!("{:?} {:?}", Path::new("a/b").cmp(Path::new("a.b")), "a/b".cmp("a.b"));
    let mut v = vec![PathBuf::from("b"), PathBuf::from("/z"), PathBuf::from("./a"), PathBuf::from("../x"), PathBuf::from("a/b"), PathBuf::from("a")];
    v.sort();
    println!("{:?}", v);
    println!("{}", Path::new("a") < Path::new("a/b"));
}
