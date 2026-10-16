// A `dyn ToString` is a pair, as a `dyn Display` is (ADR 0141): its value
// and the dictionary of its type's `ToString`, whose `to_string` gives its
// text. Of numbers, text and the crate's own `Display`, boxed, borrowed, in
// a list, and through `ToString::to_string` as a function pointer.
use std::fmt;

struct Point(i32, i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

fn describe(x: &dyn ToString) -> String {
    format!("<{}>", x.to_string())
}

fn main() {
    let items: Vec<Box<dyn ToString>> = vec![Box::new(7u8), Box::new("seven"), Box::new(Point(1, -2))];
    let texts: Vec<String> = items.iter().map(|item| item.to_string()).collect();
    println!("{texts:?}");
    println!("{} {}", describe(&2.5f64), describe(&String::from("two")));
    let shown = ToString::to_string as fn(&(dyn ToString + 'static)) -> String;
    println!("{}", shown(&Point(3, 4)));
}
