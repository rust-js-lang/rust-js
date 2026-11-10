use dep::{A, by_caller};

pub fn main() {
    println!("{}", by_caller(&A));
}
