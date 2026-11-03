// A `match` of an enum whose arms each do the same JS is that JS once,
// with no test of the variant (rd687): Rust's read of a discriminant does
// nothing, and JS's test of it may throw where Rust runs on, an
// `instanceof` of a class the host lacks. Arms that differ, in what they
// do or in where they read, are each tested.

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Line {
    from: i32,
    length: usize,
}

// Each variant's value is its payload: a struct's object, an array or a
// number.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Shape<'a> {
    Stroke(&'a Line),
    Many(&'a [i32]),
    Count(u32),
}

// A line's `length` and an array's: `shape.length`, untested.
fn length_of(shape: &Shape) -> usize {
    match shape {
        Shape::Stroke(line) => line.length,
        Shape::Many(items) => items.len(),
        Shape::Count(_) => 0,
    }
}

fn size(shape: &Shape) -> usize {
    match shape {
        Shape::Stroke(line) => line.length,
        Shape::Many(items) => items.len(),
        Shape::Count(n) => *n as usize,
    }
}

// A tagged enum's: each payload is its `_0`.
enum Tagged {
    One(Point),
    Two(Point),
}

fn sum(t: &Tagged) -> i32 {
    let p = match t {
        Tagged::One(p) => p,
        Tagged::Two(p) => p,
    };
    p.x + p.y
}

// Fields at other places aren't alike.
enum Placed {
    Front(Point, u32),
    Back(u32, Point),
}

fn placed_x(p: &Placed) -> i32 {
    match p {
        Placed::Front(p, _) => p.x,
        Placed::Back(_, p) => p.x,
    }
}

fn noted(n: i32) -> Tagged {
    println!("made {n}");
    match n % 2 {
        0 => Tagged::One(Point { x: n, y: 1 }),
        _ => Tagged::Two(Point { x: n, y: 2 }),
    }
}

// Arms that each do something, as a statement, and a subject that does
// something as it's made: made once, each arm's effect once.
fn told(t: Tagged) {
    match t {
        Tagged::One(p) => println!("at {} {}", p.x, p.y),
        Tagged::Two(p) => println!("at {} {}", p.x, p.y),
    }
}

// A subject made with effects: made once, before the arm.
fn made_x(n: i32) -> i32 {
    match noted(n) {
        Tagged::One(p) => p.x,
        Tagged::Two(p) => p.x,
    }
}

// A guard: what each arm does is its guard's test and what follows.
fn guarded(t: &Tagged) -> &'static str {
    match t {
        Tagged::One(p) if p.x > 2 => "big",
        Tagged::Two(p) if p.x > 2 => "big",
        _ => "small",
    }
}

// An `Option`'s.
fn either(o: Option<&Point>, fallback: i32) -> i32 {
    match o {
        Some(_) => fallback + 1,
        None => fallback + 1,
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// Variants whose drops differ: each tested, each dropped as Rust does.
enum Held {
    Quiet(u32),
    Noisy(Loud),
}

fn held(h: Held) -> u32 {
    match h {
        Held::Quiet(_) => 1,
        Held::Noisy(_) => 1,
    }
}

fn main() {
    let dot = Point { x: 3, y: 4 };
    let line = Line { from: 5, length: 2 };
    let many = [1, 2, 3];
    let shapes = [Shape::Stroke(&line), Shape::Many(&many), Shape::Count(4)];
    for shape in &shapes {
        println!("{} {}", length_of(shape), size(shape));
    }
    println!("{} {}", sum(&Tagged::One(Point { x: 1, y: 2 })), sum(&Tagged::Two(Point { x: 3, y: 4 })));
    println!(
        "{} {}",
        placed_x(&Placed::Front(Point { x: 7, y: 0 }, 1)),
        placed_x(&Placed::Back(2, Point { x: 8, y: 0 }))
    );
    told(noted(4));
    told(noted(5));
    println!("{} {}", made_x(8), made_x(9));
    println!("{} {} {}", guarded(&noted(1)), guarded(&noted(3)), guarded(&noted(6)));
    println!("{} {}", either(Some(&dot), 1), either(None, 2));
    println!("{} {}", held(Held::Quiet(1)), held(Held::Noisy(Loud("noisy"))));
    println!("{:?} {}", dot, line.from);
}
