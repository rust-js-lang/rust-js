// A closure's write of a `Copy` struct it took by reference, `|x| p =
// Point { x }`, is MIR's `*(_1.0) = ..`, which makes the object it
// refers to the value in place: what was copied of it before, `let q =
// p;`, is a copy of its own, as Rust's is (#469). So is a field the
// closure takes alone, `|w| frame.size = Size { w }`, a type of its own
// here, which nothing else changes in place.

#[derive(Debug, Clone, Copy)]
struct Point {
    x: i32,
}

#[derive(Debug, Clone, Copy)]
struct Size {
    w: i32,
}

#[derive(Debug)]
struct Frame {
    size: Size,
    depth: i32,
}

fn variable_captured() -> String {
    let mut p = Point { x: 0 };
    let q = p;
    let mut set = |x| p = Point { x };
    set(1);
    format!("{q:?} {p:?} {}", q.x)
}

fn field_captured() -> String {
    let mut frame = Frame {
        size: Size { w: 0 },
        depth: 10,
    };
    let q = frame.size;
    let mut set = |w| frame.size = Size { w };
    set(2);
    format!("{q:?} {frame:?} {} {}", frame.depth, q.w)
}

fn kept_in_an_array() -> String {
    let mut frame = Frame {
        size: Size { w: 0 },
        depth: 10,
    };
    let kept = [frame.size];
    let mut set = |w| frame.size = Size { w };
    set(3);
    set(4);
    format!("{kept:?} {frame:?} {}", frame.depth)
}

fn main() {
    println!("{}", variable_captured());
    println!("{}", field_captured());
    println!("{}", kept_in_an_array());
}
