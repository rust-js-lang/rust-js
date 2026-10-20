// A clone of a closure holds a copy of what it captured by value, which
// each then changes on its own: made again of a clone of it (ADR 0522).
#[derive(Clone)]
struct S(i32);

fn main() {
    let mut a = S(5);
    let mut hello = move || {
        a.0 += 1;
        a.0
    };
    let mut hello2 = hello.clone();
    println!("{} {}", hello2(), hello());
}
