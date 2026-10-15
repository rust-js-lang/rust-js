// `as_ref()` of text as a `Path`, which is its text (ADR 0173): the same JS
// value, arrayvec's `ArrayString`'s `AsRef<Path>`, `self.as_str().as_ref()`.
use std::path::Path;

struct Name {
    text: String,
}

impl AsRef<Path> for Name {
    fn as_ref(&self) -> &Path {
        self.text.as_str().as_ref()
    }
}

fn ext<P: AsRef<Path>>(p: P) -> Option<String> {
    p.as_ref().extension().and_then(|e| e.to_str()).map(String::from)
}

fn main() {
    let n = Name { text: "a/b.rs".to_string() };
    let p: &Path = n.as_ref();
    println!("{}", p.display());
    println!("{:?}", ext(n));
}
