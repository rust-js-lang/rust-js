// A `Cow` compares and orders as what it borrows, as std's impls deref it:
// a borrowed one equal to an owned one of the same text, beside a `&str`
// or a `String` too, and in what derives its `==` (ADR 0400).
use std::borrow::Cow;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Label {
    rank: u8,
    name: Cow<'static, str>,
}

fn label(rank: u8, name: Cow<'static, str>) -> Label {
    Label { rank, name }
}

fn main() {
    let borrowed: Cow<'static, str> = Cow::Borrowed("apple");
    let owned: Cow<'static, str> = Cow::Owned("apple".to_string());
    println!("{} {}", borrowed == owned, borrowed != owned);
    println!("{} {} {}", owned == "apple", "apple" == owned, borrowed == "pear");
    println!("{}", &owned == &"apple");
    let text = String::from("apple");
    println!("{} {}", borrowed == text, text == borrowed);
    println!("{:?}", borrowed.cmp(&Cow::Owned("banana".to_string())));
    println!("{}", Cow::Borrowed("b") > Cow::<str>::Owned("a".to_string()));

    let mut words: Vec<Cow<'static, str>> = vec![
        Cow::Owned("pear".to_string()),
        Cow::Borrowed("apple"),
        Cow::Owned("fig".to_string()),
        Cow::Borrowed("banana"),
    ];
    words.sort();
    println!("{:?}", words);
    println!("{:?}", words.iter().max());

    let some: Option<Cow<'static, str>> = Some(Cow::Borrowed("x"));
    println!("{}", some == Some(Cow::Owned("x".to_string())));

    let a = label(1, Cow::Borrowed("one"));
    let b = label(1, Cow::Owned("one".to_string()));
    let c = label(1, Cow::Borrowed("two"));
    println!("{} {} {:?}", a == b, a < c, b.cmp(&c));

    let nums: Cow<'static, [i32]> = Cow::Borrowed(&[1, 2, 3]);
    let more: Cow<'static, [i32]> = Cow::Owned(vec![1, 2, 3]);
    println!("{} {:?}", nums == more, nums.cmp(&Cow::Owned(vec![1, 2, 4])));
}
