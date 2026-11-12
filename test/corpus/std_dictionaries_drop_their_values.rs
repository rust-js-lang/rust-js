//@ library-differs
// A `dyn` of a std impl's dictionary, `ToString`'s blanket one, std's
// `Debug` of a `Vec` and `Display` of an `Rc` or a `Box`, carries its
// type's drop, as an impl's own dictionary does: dropping the `Box<dyn ..>`
// drops what it holds (#1578).
use std::fmt;
#[derive(Debug, Clone, PartialEq, Default)]
struct Point(i32);
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Drop for Point {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let items: Vec<Box<dyn ToString>> = vec![Box::new(Point(1))];
    println!("{}", items.len());
    drop(items);
    let shown: Box<dyn fmt::Display> = Box::new(Point(2));
    println!("{shown}");
    drop(shown);
    let debugged: Box<dyn fmt::Debug> = Box::new(Point(3));
    println!("{debugged:?}");
    drop(debugged);
    let any: Box<dyn std::any::Any> = Box::new(Point(4));
    println!("{}", any.is::<Point>());
    drop(any);
    let r: &dyn ToString = &Point(5);
    println!("{}", r.to_string());
    let f: fn(&dyn ToString) -> String = |s| s.to_string();
    println!("{}", f(&Point(6)));
    let to_text = ToString::to_string as fn(&(dyn ToString + 'static)) -> String;
    println!("{}", to_text(&Point(7)));
    let list: Box<dyn fmt::Debug> = Box::new(vec![Point(8), Point(9)]);
    println!("{list:?}");
    drop(list);
    let counted: Box<dyn fmt::Display> = Box::new(std::rc::Rc::new(Point(10)));
    println!("{counted}");
    drop(counted);
    let boxed: Box<dyn fmt::Display> = Box::new(Box::new(Point(11)));
    println!("{boxed}");
    drop(boxed);
    let texts: Box<dyn ToString> = Box::new(std::rc::Rc::new(Point(12)));
    println!("{}", texts.to_string());
    drop(texts);
    println!("end");
}
