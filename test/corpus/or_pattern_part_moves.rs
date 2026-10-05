// An or-pattern that moves part of a value with a destructor: what each
// alternative leaves is dropped, as chrono's `MappedLocalTime::earliest` and
// `latest` leave the other end of an ambiguous time.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum Mapped<T> {
    None,
    Single(T),
    Ambiguous(T, T),
}

impl<T> Mapped<T> {
    fn earliest(self) -> Option<T> {
        match self {
            Mapped::Single(t) | Mapped::Ambiguous(t, _) => Some(t),
            _ => None,
        }
    }

    fn latest(self) -> Option<T> {
        match self {
            Mapped::Single(t) | Mapped::Ambiguous(_, t) => Some(t),
            _ => None,
        }
    }
}

enum Shape {
    Pair(Loud, Loud),
    Triple(Loud, Loud, Loud),
    Empty,
}

fn middle(shape: Shape) -> Option<Loud> {
    if let Shape::Pair(_, m) | Shape::Triple(_, m, _) = shape {
        println!("got {}", m.0);
        return Some(m);
    }
    println!("none");
    None
}

fn name(shape: Shape) -> &'static str {
    let (Shape::Pair(first, _) | Shape::Triple(_, _, first)) = shape else {
        return "empty";
    };
    println!("kept {}", first.0);
    first.0
}

fn main() {
    let a = Mapped::Ambiguous(Loud("a1"), Loud("a2")).earliest();
    println!("earliest {}", a.as_ref().map_or("none", |l| l.0));
    let b = Mapped::Ambiguous(Loud("b1"), Loud("b2")).latest();
    println!("latest {}", b.as_ref().map_or("none", |l| l.0));
    let c = Mapped::Single(Loud("c")).earliest();
    println!("single {}", c.as_ref().map_or("none", |l| l.0));
    let n: Option<Loud> = Mapped::None.latest();
    println!("none {}", n.is_none());
    let numbers = Mapped::Ambiguous(1, 2).latest();
    println!("numbers {numbers:?}");

    let m = middle(Shape::Triple(Loud("t1"), Loud("t2"), Loud("t3")));
    println!("middle {}", m.as_ref().map_or("none", |l| l.0));
    let p = middle(Shape::Pair(Loud("p1"), Loud("p2")));
    println!("middle {}", p.as_ref().map_or("none", |l| l.0));
    println!("middle none {}", middle(Shape::Empty).is_none());

    println!("name {}", name(Shape::Triple(Loud("x1"), Loud("x2"), Loud("x3"))));
    println!("name {}", name(Shape::Pair(Loud("y1"), Loud("y2"))));
    println!("name {}", name(Shape::Empty));
    println!("end");
}
