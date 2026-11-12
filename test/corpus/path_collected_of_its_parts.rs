// A path collected of its parts, `PathBuf`'s `FromIterator`, each pushed in
// turn: quickcheck 0.9's `PathBuf` shrinker normalizes one so (#1420).
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

fn normalized(p: &Path) -> PathBuf {
    p.iter().collect::<PathBuf>()
}

fn main() {
    for text in ["a/b//c/", "/usr/./lib", "", "x", "/", "a/../b"] {
        let p = PathBuf::from(text);
        let n = normalized(&p);
        println!("{text:?} {n:?} {}", n.as_os_str() != p.as_os_str());
    }
    let parts = ["usr", "local", "/etc", "conf"];
    let joined: PathBuf = parts.iter().collect();
    println!("{joined:?}");
    let owned: PathBuf = vec![String::from("a"), String::from("b")].into_iter().collect();
    println!("{owned:?}");
    let from_os: PathBuf = [OsStr::new("x"), OsStr::new("y/")].into_iter().collect();
    println!("{from_os:?}");
}
