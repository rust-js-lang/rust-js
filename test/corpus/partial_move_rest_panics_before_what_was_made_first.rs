// What a partial move leaves, dropped where its scope ends, panics, and
// what was made before it is still dropped as the panic unwinds: the rest's
// drop runs something in JS, so its cleanup is written (#2112's change D3).
// Of a struct's fields no pattern bound, and of an enum's variants none
// took apart.
use std::panic;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0.ends_with('!') && !std::thread::panicking() {
            panic!("{} panicked", self.0);
        }
    }
}

struct Three(Loud, Loud, Loud);

enum Shape {
    Pair(Loud, Loud),
    Triple(Loud, Loud, Loud),
}

fn last_of_three([a, b, c]: [&'static str; 3]) -> &'static str {
    let _earlier = Loud("earlier");
    let three = Three(Loud(a), Loud(b), Loud(c));
    let Three(_, _, last) = three;
    last.0
}

fn first_of_shape(shape: fn() -> Shape) -> &'static str {
    let _earlier = Loud("earlier shape");
    let shape = shape();
    let (Shape::Pair(first, _) | Shape::Triple(_, _, first)) = shape;
    first.0
}

fn main() {
    let r = panic::catch_unwind(|| last_of_three(["a!", "b", "c"]));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| last_of_three(["d", "e", "f"]));
    println!("ok {:?}", r.ok());
    let r = panic::catch_unwind(|| first_of_shape(|| Shape::Triple(Loud("g!"), Loud("h"), Loud("i"))));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| first_of_shape(|| Shape::Pair(Loud("j"), Loud("k!"))));
    println!("caught {}", r.is_err());
}
