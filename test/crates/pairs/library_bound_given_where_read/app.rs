//! A consumer of a library's generic functions, which gives each the
//! dictionaries its contract says it reads (#1915).
#[derive(Debug)]
struct Point(u8, u8);

pub fn main() {
    let (point, label) = dep::labeled(Point(1, 2), "origin");
    println!("{label} {}", dep::shown(&point));
}
