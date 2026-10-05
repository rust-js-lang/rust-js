// A `Box<str>` is its string, as a `String` is: one made from a `&str` or a
// `String`, as chrono's `Item::to_owned` boxes a literal.
#[derive(Debug, Clone, PartialEq)]
enum Item<'a> {
    Literal(&'a str),
    OwnedLiteral(Box<str>),
}

impl Item<'_> {
    fn to_owned(self) -> Item<'static> {
        match self {
            Item::Literal(s) => Item::OwnedLiteral(Box::from(s)),
            Item::OwnedLiteral(s) => Item::OwnedLiteral(s),
        }
    }

    fn text(&self) -> &str {
        match self {
            Item::Literal(s) => s,
            Item::OwnedLiteral(s) => s,
        }
    }
}

fn main() {
    let owned = Item::Literal("abc").to_owned();
    println!("{owned:?} {} {}", owned.text(), owned.text().len());
    println!("{}", owned == Item::OwnedLiteral(Box::from("abc")));
    let from_string: Box<str> = Box::from(String::from("héllo"));
    let into: Box<str> = "into".into();
    println!("{from_string} {} {into:?} {}", from_string.len(), into.to_uppercase());
    let copy = from_string.clone();
    println!("{} {}", copy == from_string, &*copy == "héllo");
}
