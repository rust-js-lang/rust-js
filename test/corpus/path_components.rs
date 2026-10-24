// A path's components (ADR 0700): std's `Component`, as any enum is,
// shown, compared, ordered, hashed and given as a path; and std's other
// enums whose `PartialEq` is derived, `Bound`'s, compared.
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::ops::Bound;
use std::path::{Component, Path, PathBuf};

fn kind(c: Component) -> String {
    match c {
        Component::RootDir => "root".into(),
        Component::CurDir => "cur".into(),
        Component::ParentDir => "parent".into(),
        Component::Normal(s) => format!("n:{}", s.to_str().unwrap()),
        Component::Prefix(_) => "prefix".into(),
    }
}

fn show<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().display().to_string()
}

fn main() {
    let made = [Component::RootDir, Component::Normal(OsStr::new("usr")), Component::ParentDir];
    for c in made {
        println!("{} {:?} {:?}", kind(c), c, c.as_os_str());
    }
    for p in ["/usr//lib/./x.so/", "./a/../b", "", "a/./.", "/", "../constructor", "toString"] {
        let path = Path::new(p);
        let all: Vec<Component> = path.components().collect();
        println!("{p:?} {all:?} {:#?}", path.components().next());
        let mut ends = path.components();
        println!("{:?} {:?}", ends.next_back(), ends.next());
        let texts: Vec<&str> = path.components().rev().map(|c| c.as_os_str().to_str().unwrap()).collect();
        println!("{}", texts.join("|"));
        println!("{}", path.components().filter(|c| matches!(c, Component::Normal(_))).count());
        println!("{:?}", path.iter().next_back());
    }
    let x = Path::new("x");
    println!("{}", x.components().any(|c| c == Component::Normal(OsStr::new("x"))));

    // Ordered, by variant then text; kept in sets.
    let p = Path::new("/b/a/../c/a");
    let mut sorted: Vec<Component> = p.components().collect();
    sorted.sort();
    println!("{sorted:?} {:?}", Component::RootDir.cmp(&Component::ParentDir));
    let hashed: HashSet<Component> = p.components().collect();
    let ordered: BTreeSet<Component> = p.components().collect();
    println!("{} {ordered:?}", hashed.len());

    // Given as a path, or as an `OsStr`.
    let mut kept = PathBuf::new();
    for c in Path::new("/a/./b/../c").components() {
        if c != Component::ParentDir {
            kept.push(c);
        }
    }
    let y = Component::Normal(OsStr::new("y"));
    println!("{} {} {}", kept.display(), x.join(y).display(), Path::new(&y).display());
    println!("{:?} {} {}", OsStr::new(&y), show(y), show(Component::RootDir));

    // An `OsStr`'s order, by its bytes.
    let mut texts = vec![OsStr::new("\u{ff61}"), OsStr::new("\u{1F600}"), OsStr::new("b"), OsStr::new("a")];
    texts.sort();
    println!("{texts:?} {}", OsStr::new("a") < OsStr::new("b"));

    // `Bound`'s derived `==`.
    let a: Bound<u32> = Bound::Included(3);
    println!("{} {} {:?}", a == Bound::Excluded(3), a == Bound::Included(3), a);
}
