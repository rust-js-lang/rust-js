// A path's methods go over its components as Rust's `Components` does on
// Unix: `parent()` is its slice before its last one, `file_name()` that
// one if it's a name, and a `PathBuf` changed in place is its text's edit
// (ADR 0405).
use std::path::{Path, PathBuf};

fn main() {
    let cases = [
        "a/b/c", "/a", "/", "", "a", "./a", ".", "a/./b", "a/b/", "//a", "a/..", "../x", "x/a.tar.gz//",
        ".bashrc", "a.", "..", "./", "src/main.rs", "/usr/lib/libc.so.6", "dir/.hidden.txt",
    ];
    for case in cases {
        let p = Path::new(case);
        println!(
            "{:?}: parent {:?} name {:?} stem {:?} ext {:?} abs {} rel {} root {}",
            p,
            p.parent(),
            p.file_name(),
            p.file_stem(),
            p.extension(),
            p.is_absolute(),
            p.is_relative(),
            p.has_root()
        );
        println!("  with_extension {:?} with_file_name {:?}", p.with_extension("md"), p.with_file_name("x.y"));
    }
    println!("{:?} {:?} {:?}", Path::new("a").join("b"), Path::new("a/").join("b"), Path::new("a").join("/b"));
    println!("{:?} {:?}", Path::new("").join("b"), Path::new("/").join("etc").join("hosts"));
    let base = Path::new("/usr/lib/rust");
    for other in ["/usr", "/usr/lib", "/usr/li", "usr", "/usr/lib/rust/", "/usr//lib", "/"] {
        println!("starts {:?} {}", other, base.starts_with(other));
    }
    for other in ["rust", "lib/rust", "ib/rust", "/usr/lib/rust", "/lib/rust", ""] {
        println!("ends {:?} {}", other, base.ends_with(other));
    }

    for case in ["/usr/lib/../x/./y.txt", "a//b/", "./c", "", "/"] {
        let p = Path::new(case);
        let parts: Vec<&str> = p.iter().map(|part| part.to_str().unwrap()).collect();
        println!("iter {:?} {:?} {}", parts, p.ancestors().collect::<Vec<_>>(), p.iter().count());
    }
    let mut buf = PathBuf::from("/tmp");
    buf.push("cache");
    buf.push("item.json");
    println!("{:?}", buf);
    println!("{} {:?}", buf.set_extension("bak"), buf);
    println!("{} {:?}", buf.set_extension(""), buf);
    buf.set_file_name("other.txt");
    println!("{:?}", buf);
    println!("{} {:?}", buf.pop(), buf);
    println!("{} {} {:?}", buf.pop(), buf.pop(), buf);
    println!("{} {:?}", buf.pop(), buf);
    buf.push("/abs");
    println!("{:?} {}", buf, buf.to_path_buf() == PathBuf::from("/abs"));
    let mut empty = PathBuf::new();
    println!("{} {}", empty.set_extension("x"), empty.pop());
    empty.set_file_name("named");
    println!("{:?}", empty);
}
