// A `String` from the crate's own type is its impl's, `From<&W> for
// String`; std's from text, a `Box<str>` and a `Cow<str>` is the text.
use std::borrow::Cow;

struct Words {
    items: Vec<String>,
}

impl<'a> From<&'a Words> for String {
    fn from(words: &'a Words) -> String {
        words.items.join("+")
    }
}

struct Code(u8);

impl From<Code> for String {
    fn from(code: Code) -> String {
        format!("c{}", code.0)
    }
}

fn main() {
    let words = Words { items: vec!["a".to_string(), "b".to_string()] };
    let joined: String = (&words).into();
    println!("{joined} {}", String::from(&words));
    let code: String = Code(2).into();
    println!("{} {code}", String::from(Code(1)));

    let boxed: Box<str> = "boxed".into();
    let borrowed: Cow<str> = Cow::Borrowed("cow");
    let owned: Cow<str> = Cow::Owned("owned".to_string());
    let from_owned: String = owned.into();
    println!("{} {} {from_owned} {}", String::from(boxed), String::from(borrowed), String::from('x'));
}
