// A `dyn Any` and its downcasts: `is`, `downcast_ref`, `downcast_mut`,
// `Box::downcast`, `TypeId`, `type_name`, and a trait whose supertrait is
// `Any`, upcast to it.

use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
}

trait Component: Any {
    fn name(&self) -> &str;
}

impl Component for Point {
    fn name(&self) -> &str {
        "point"
    }
}

fn describe(value: &dyn Any) -> String {
    if let Some(n) = value.downcast_ref::<i32>() {
        format!("i32 {n}")
    } else if let Some(s) = value.downcast_ref::<String>() {
        format!("string {s}")
    } else if value.is::<Point>() {
        format!("point {:?}", value.downcast_ref::<Point>())
    } else {
        String::from("other")
    }
}

fn key<T: Any>(_: &T) -> TypeId {
    TypeId::of::<T>()
}

fn boxed<T: Any>(value: T) -> Box<dyn Any> {
    Box::new(value)
}

fn main() {
    let items: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(String::from("s")), Box::new(Point { x: 1 }), Box::new(1.5f64)];
    for item in &items {
        println!("{}", describe(item.as_ref()));
    }
    let mut number: Box<dyn Any> = Box::new(7u8);
    if let Some(n) = number.downcast_mut::<u8>() {
        *n += 1;
    }
    println!("{:?} {:?}", number.downcast_ref::<u8>(), number.downcast_ref::<i8>());
    match number.downcast::<u8>() {
        Ok(n) => println!("u8 {n}"),
        Err(_) => println!("not u8"),
    }
    let text = boxed(String::from("t"));
    match text.downcast::<i32>() {
        Ok(n) => println!("i32 {n}"),
        Err(back) => println!("back {:?}", back.downcast_ref::<String>()),
    }
    println!("{} {}", TypeId::of::<i32>() == key(&3), key(&3u8) == TypeId::of::<i32>());
    let mut seen: HashMap<TypeId, &str> = HashMap::new();
    seen.insert(TypeId::of::<Point>(), "point");
    seen.insert(5i64.type_id(), "i64");
    println!("{:?} {}", seen.get(&TypeId::of::<i64>()), seen.len());
    println!("{} {} {}", type_name::<Point>().ends_with("::Point"), type_name::<Vec<Option<&str>>>(), type_name::<f64>());
    let component: Box<dyn Component> = Box::new(Point { x: 2 });
    let any: &dyn Any = component.as_ref();
    println!("{} {:?} {:?}", component.name(), any.downcast_ref::<Point>(), any);
    println!("{}", (*component).type_id() == TypeId::of::<Point>());
}
