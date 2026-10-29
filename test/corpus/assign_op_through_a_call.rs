// A compound assignment of a borrowed operand, `+= &five`, through the
// `&mut` a call gives: the call runs once, before the operand, as Rust
// runs it, and the place it gives is read and written; and through one
// held, `let held = first(&mut v)`.

struct Counter {
    hits: i32,
    name: String,
}

fn first(v: &mut [i32]) -> &mut i32 {
    println!("first");
    &mut v[0]
}

fn hits(c: &mut Counter) -> &mut i32 {
    println!("hits");
    &mut c.hits
}

fn name(c: &mut Counter) -> &mut String {
    println!("name");
    &mut c.name
}

fn five() -> i32 {
    println!("five");
    5
}

fn bump(r: &mut i32, by: &i32) {
    *r += by;
}

fn shout(s: &mut String) {
    *s += "!";
}

fn main() {
    let mut v = vec![1, 2];
    *first(&mut v) += &five();
    *first(&mut v) -= &2;
    let held = first(&mut v);
    *held += &five();
    let mut c = Counter {
        hits: 1,
        name: "a".to_string(),
    };
    *hits(&mut c) *= &3;
    *name(&mut c) += "b";
    *name(&mut c) += &String::from("c");
    bump(first(&mut v), &five());
    shout(name(&mut c));
    println!("{v:?} {} {}", c.hits, c.name);
}
