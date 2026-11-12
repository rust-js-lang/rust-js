// `type_name` of a type that holds a type only a caller knows: a type
// parameter, `Vec<T>`, or an associated type of one, `Option<I::Item>`, a
// std trait's or the crate's, each one's name its caller's in its place
// (#1603). An associated type in a slice was named `!`, in an `Option` the
// compiler crashed. One that is a type here, `<Vec<u8> as
// IntoIterator>::Item`, is its type.
use std::any::type_name;
use std::borrow::Cow;

trait Source {
    type Item;
}

struct Words;

impl Source for Words {
    type Item = String;
}

fn of_item<I: Iterator>(_: &I) -> &'static str {
    type_name::<Option<I::Item>>()
}

fn of_items<I: IntoIterator>(_: &I) -> &'static str
where
    I::Item: Clone + 'static,
{
    type_name::<Cow<'static, [I::Item]>>()
}

fn of_both<T, I: Iterator>(_: &T, _: &I) -> &'static str {
    type_name::<(T, Vec<I::Item>, T)>()
}

fn of_own<S: Source>(_: &S) -> &'static str {
    type_name::<Option<(S::Item, u8)>>()
}

fn of_known<T>(_: &T) -> &'static str {
    type_name::<Option<(T, <Vec<u8> as IntoIterator>::Item)>>()
}

fn of_nested<I: IntoIterator>(_: &I) -> &'static str
where
    I::Item: IntoIterator,
{
    type_name::<Vec<<I::Item as IntoIterator>::Item>>()
}

fn main() {
    println!("{}", of_item(&[1u8, 2].into_iter()));
    println!("{}", of_item(&"ab".chars()));
    println!("{}", of_items(&vec![String::new()]));
    println!("{}", of_both(&1i64, &vec![true].into_iter()));
    println!("{}", of_own(&Words));
    println!("{}", of_known(&'c'));
    println!("{}", of_nested(&vec![vec![1u16]]));
}
