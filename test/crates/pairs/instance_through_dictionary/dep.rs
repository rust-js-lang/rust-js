//! A trait's generic default, which the library's own impl's dictionary
//! gives, and a private function it calls (#483).
pub trait Shout {
    fn word(&self) -> String;

    fn shout<T: AsRef<str>>(&self, end: T) -> String {
        loud(&self.word(), end.as_ref())
    }
}

fn loud(word: &str, end: &str) -> String {
    format!("{}{end}", word.to_uppercase())
}

pub struct Dog;

impl Shout for Dog {
    fn word(&self) -> String {
        String::from("woof")
    }
}
