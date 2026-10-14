// A `match` of a call's value evaluates the call once, however many arms
// test it: `match note(15, x) { 1 => .., 2 => .., _ => .. }` printed its
// note once per arm tested. Found by a generated program (seed 50147).

fn id<T>(x: T) -> T {
    x
}

fn note<T>(tag: u32, x: T) -> T {
    println!("note {tag}");
    x
}

fn pick(x: i32) -> i32 {
    match note(1, id(x)) {
        1 => 10,
        2 | 3 => 20,
        4..=9 => 30,
        n if n < 0 => 40,
        _ => 50,
    }
}

fn label(x: u8) -> &'static str {
    match note(2, x) {
        b'a' => "a",
        b'b' => "b",
        b'c' => "c",
        _ => "other",
    }
}

fn main() {
    for x in [1, 3, 5, -1, 100] {
        println!("{}", pick(x));
    }
    println!("{:<4}|", match note(3, id(5i32)) { 1 => 10, 2 => 20, _ => 30 });
    let v = match note(4, id(2u64)) {
        0 => "zero",
        1 => "one",
        2 => "two",
        _ => "many",
    };
    println!("{v} {} {}", label(b'b'), label(b'z'));
}
