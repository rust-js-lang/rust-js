// What a partial move leaves is dropped as Rust's drop glue drops it: each
// field after one whose drop panics is still dropped as the panic unwinds,
// of a struct, of an enum's variant and of an array, as the whole value's
// drop already drops them (#2146).
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

fn last_of_three(three: Three) -> &'static str {
    let Three(_, _, last) = three;
    last.0
}

fn first_of_shape(shape: Shape) -> &'static str {
    let (Shape::Pair(first, _) | Shape::Triple(_, _, first)) = shape;
    first.0
}

fn middle_of_array(items: [Loud; 4]) -> &'static str {
    let [_, middle, ..] = items;
    middle.0
}

fn whole(three: Three) -> usize {
    let n = three.0 .0.len();
    drop(three);
    n
}

fn main() {
    let _outer = Loud("outer");
    let r = panic::catch_unwind(|| last_of_three(Three(Loud("a!"), Loud("b"), Loud("c"))));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| last_of_three(Three(Loud("d"), Loud("e"), Loud("f"))));
    println!("ok {:?}", r.ok());
    let r = panic::catch_unwind(|| first_of_shape(Shape::Triple(Loud("g!"), Loud("h"), Loud("i"))));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| first_of_shape(Shape::Pair(Loud("j"), Loud("k!"))));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| middle_of_array([Loud("l!"), Loud("m"), Loud("n"), Loud("o")]));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| middle_of_array([Loud("p"), Loud("q"), Loud("r!"), Loud("s")]));
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| whole(Three(Loud("t!"), Loud("u"), Loud("v"))));
    println!("caught {}", r.is_err());
    println!("end");
}
