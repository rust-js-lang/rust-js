// A path's method given a type of the crate's own that's an `AsRef<Path>`
// or an `AsRef<OsStr>` reads its `as_ref()`, through its impl, never the
// struct itself (#1388): `join`, `push`, `starts_with`, `strip_prefix`,
// `with_file_name` and `set_extension`, of a value and of a `&` of one.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

struct Name {
    text: String,
}

impl AsRef<Path> for Name {
    fn as_ref(&self) -> &Path {
        println!("as_ref of {}", self.text);
        self.text.as_str().as_ref()
    }
}

impl AsRef<OsStr> for Name {
    fn as_ref(&self) -> &OsStr {
        OsStr::new(self.text.as_str())
    }
}

fn name(text: &str) -> Name {
    Name { text: text.to_string() }
}

fn main() {
    let base = Path::new("/usr");
    let lib = name("lib");
    println!("{}", base.join(&lib).display());
    println!("{}", base.join(name("/etc")).display());
    let mut p = PathBuf::from("/a");
    p.push(name("b"));
    p.push(&lib);
    println!("{}", p.display());
    println!("{}", base.starts_with(name("/usr")));
    println!("{}", p.ends_with(&lib));
    println!("{:?}", p.strip_prefix(name("/a")).map(|rest| rest.display().to_string()));
    println!("{}", base.with_file_name(&lib).display());
    p.set_extension(name("rs"));
    println!("{}", p.display());
    println!("{}", Path::new(&lib).display());
    // A file's path, opened alone or by options (#1156).
    println!("{}", std::fs::File::open(name("/no/such/file")).is_err());
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    let missing = name("/no/such/lib");
    println!("{}", options.open(&missing).is_err());
}
