//! The library's generic function as a value of one type.
pub fn main() {
    let f: fn(String) -> (String, String) = dep::duplicate::<String>;
    println!("{:?}", f("a".to_string()));
    let pairs: Vec<(u32, u32)> = [1u32, 2].into_iter().map(dep::duplicate).collect();
    println!("{:?}", pairs);
    let add = dep::add_to::<u8>;
    let mut total = 1;
    add(&mut total, 3);
    println!("{}", total);
    // Kept as a value, called twice and given on: its arrow gives the
    // library the box it's given, which the library writes.
    add(&mut total, 4);
    let mut other = 10;
    each(add, &mut other);
    println!("{} {}", total, other);
}

fn each(f: fn(&mut u32, u8), n: &mut u32) {
    f(n, 1);
    f(n, 2);
}
