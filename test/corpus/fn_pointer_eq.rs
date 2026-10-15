// `==` of two function pointers is JS's identity, as Rust's is of their
// addresses: the same function's are equal, others' aren't (ADR 0262's
// rare and harmless: rustc may merge two functions of the same code, or
// split one, which JS never does).
fn double(x: i32) -> i32 {
    x * 2
}

fn square(x: i32) -> i32 {
    x * x
}

#[allow(unpredictable_function_pointer_comparisons)]
fn main() {
    let f: fn(i32) -> i32 = double;
    let g: fn(i32) -> i32 = square;
    let table = [double as fn(i32) -> i32, square];
    println!("{} {} {}", f == table[0], f == g, g != table[1]);
    let found = table.iter().position(|&h| h == g);
    println!("{found:?}");
    // Through references, `PartialEq`'s, and in an `Option`.
    println!("{} {}", &f == &table[0], Some(g) == Some(square as fn(i32) -> i32));
}
