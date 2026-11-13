//@ library-differs
// `parse` to a type only a caller knows: an associated type's, `S::Item`
// or an `IntoIterator`'s `I::Item`, is its dictionary's `from_str`, as a
// type parameter's is (#1605). Std's numbers and `bool`, and the crate's own.
use std::fmt::Debug;
use std::str::FromStr;

trait Source {
    type Item;
}

struct Nums;

impl Source for Nums {
    type Item = u8;
}

struct Words;

impl Source for Words {
    type Item = Word;
}

#[derive(Debug)]
struct Word(String);

impl FromStr for Word {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.is_empty() {
            true => Err("empty".to_string()),
            false => Ok(Word(text.to_uppercase())),
        }
    }
}

fn parse_item<S: Source>(_: &S, text: &str) -> Result<S::Item, <S::Item as FromStr>::Err>
where
    S::Item: FromStr,
{
    text.parse()
}

fn parse_all<I>(texts: &[&str]) -> Vec<Option<I::Item>>
where
    I: IntoIterator,
    I::Item: FromStr,
{
    texts.iter().map(|text| text.parse::<I::Item>().ok()).collect()
}

fn shown<S: Source>(source: &S, text: &str) -> String
where
    S::Item: FromStr + Debug,
    <S::Item as FromStr>::Err: Debug,
{
    format!("{:?}", parse_item(source, text))
}

pub fn main() {
    println!("{:?}", parse_item(&Nums, "300").is_err());
    println!("{:?}", parse_item(&Nums, "30"));
    println!("{}", shown(&Nums, "-1"));
    println!("{}", shown(&Words, "abc"));
    println!("{}", shown(&Words, ""));
    if let Ok(word) = parse_item(&Words, "hi") {
        println!("{}", word.0);
    }
    println!("{:?}", parse_all::<Vec<i64>>(&["1", "x", "-9007199254740993"]));
    println!("{:?}", parse_all::<Option<f32>>(&["1.5", "nan", "e"]));
    println!("{:?}", parse_all::<[bool; 2]>(&["true", "no"]));
}
