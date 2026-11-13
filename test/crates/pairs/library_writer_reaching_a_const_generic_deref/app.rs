//! A program that shows the library's const-generic writer (#1608).
pub fn main() {
    println!("{:?}", dep::stack::<4>(vec![1, 2]));
}
