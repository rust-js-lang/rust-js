// `TypeId`s and downcasts of generic code: a `T: 'static`'s `TypeId`, given
// by its caller, a downcast to a type parameter, an `Rc<dyn Any>` and a
// `dyn Any + Send` (ADR 0331).

use std::any::{Any, TypeId};
use std::rc::Rc;

fn same<T: 'static, U: 'static>() -> bool {
    TypeId::of::<T>() == TypeId::of::<U>()
}

fn relay<T: 'static>() -> bool {
    same::<T, u8>()
}

fn find<T: 'static + Clone>(items: &[Box<dyn Any>]) -> Vec<T> {
    items.iter().filter_map(|item| item.downcast_ref::<T>().cloned()).collect()
}

fn erased<T: 'static>(value: T) -> Box<dyn Any> {
    Box::new(value)
}

fn first(text: &String) -> &str {
    text
}

fn made() -> Box<dyn Any> {
    println!("made");
    Box::new(1)
}

trait AsStr<'a, 'b> {
    fn get(&'a self) -> &'b str;
}

impl<'a> AsStr<'a, 'a> for String {
    fn get(&'a self) -> &'a str {
        self
    }
}

fn main() {
    println!("{} {} {}", same::<i32, i32>(), same::<i32, i64>(), relay::<u8>());
    let items: Vec<Box<dyn Any>> = vec![Box::new(1u8), Box::new("a"), Box::new(2u8), erased(String::from("e"))];
    println!("{:?} {:?} {:?}", find::<u8>(&items), find::<&str>(&items), find::<String>(&items));
    let shared: Rc<dyn Any> = Rc::new(vec![1, 2]);
    match shared.downcast::<Vec<i32>>() {
        Ok(list) => println!("{list:?}"),
        Err(_) => println!("not a list"),
    }
    let sent: Box<dyn Any + Send> = Box::new(Some(3u16));
    println!("{:?} {}", sent.downcast_ref::<Option<u16>>(), sent.is::<u16>());
    let none: Box<dyn Any> = Box::new(None::<u16>);
    println!("{:?}", none.downcast_ref::<Option<u16>>());
    let f: Box<dyn Any> = Box::new(first as fn(&String) -> &str);
    println!("{} {}", f.is::<fn(&String) -> &str>(), f.is::<fn(&String) -> &'static str>());
    println!("{:?}", made());
    let held: Box<dyn Any> = Box::new(Box::new(String::from("h")) as Box<dyn for<'a> AsStr<'a, 'a>>);
    println!("{} {}", held.is::<Box<dyn for<'a> AsStr<'a, 'a>>>(), held.is::<Box<dyn for<'a> AsStr<'a, 'static>>>());
}
