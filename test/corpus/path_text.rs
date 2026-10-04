// A `Path` or a `PathBuf` is its text, as a `String` is: made from text,
// shown by `display()`, and read back by `to_str()`, as thiserror's
// `AsDisplay` does.

use std::fmt::Display;
use std::path::{Path, PathBuf};

trait AsDisplay<'a> {
    type Target: Display;
    fn as_display(&'a self) -> Self::Target;
}

impl<'a> AsDisplay<'a> for Path {
    type Target = std::path::Display<'a>;
    fn as_display(&'a self) -> Self::Target {
        self.display()
    }
}

struct Missing {
    path: PathBuf,
}

fn describe(e: &Missing) -> String {
    format!("no file at {}", e.path.as_display())
}

fn main() {
    let owned = PathBuf::from(String::from("config/app.toml"));
    let borrowed = Path::new("logs");
    println!("{} {}", owned.display(), borrowed.display());
    println!("{:?} {:?}", owned, borrowed);
    println!("{:?} {:?}", owned.to_str(), borrowed.as_os_str().to_str());
    let copy = owned.clone();
    let as_path: &Path = copy.as_path();
    println!("{}", as_path.display());
    println!("{}", describe(&Missing { path: PathBuf::from("data/x.csv") }));
    println!("{}", PathBuf::new().display());
}
