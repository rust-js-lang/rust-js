//! Shows its own types by the library's generic writer function, plain and
//! pretty, none, one and many fields (#1026).
use std::fmt;

struct Point(i32);

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

struct Points(Vec<Point>);

impl fmt::Debug for Points {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        dep::rows("Points", &self.0, f)
    }
}

pub fn main() {
    println!("{:?}", Points(vec![]));
    println!("{:?}", Points(vec![Point(1), Point(2)]));
    println!("{:#?}", Points(vec![Point(3)]));
    println!("{:?}", Points(vec![Point(4), Point(5), Point(6)]));
}
