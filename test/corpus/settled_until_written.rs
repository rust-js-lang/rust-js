// What a variant binds of a `mut` parameter is read in place where the
// parameter isn't written between, though it's written after (#121): not
// where it's written before the binding's read, in an arm or a loop's next
// turn, nor where a closure takes the binding, which reads it when called.

#[derive(Debug, Clone, Copy)]
enum Shape {
    Line(i32),
    Dot,
}

fn after(mut shape: Shape) -> String {
    let text = match shape {
        Shape::Line(n) if n > 10 => format!("long {n}"),
        Shape::Line(n) => format!("line {n}"),
        Shape::Dot => "dot".to_string(),
    };
    shape = Shape::Dot;
    format!("{text} then {shape:?}")
}

fn between(mut shape: Shape) -> i32 {
    match shape {
        Shape::Line(n) => {
            shape = Shape::Line(n + 100);
            println!("now {shape:?}");
            n
        }
        Shape::Dot => 0,
    }
}

fn taken(mut shape: Shape) -> i32 {
    let read = match shape {
        Shape::Line(n) => move || n,
        Shape::Dot => return 0,
    };
    shape = Shape::Line(7);
    println!("{shape:?}");
    read()
}

fn turns(mut shape: Shape) -> i32 {
    let mut total = 0;
    for _ in 0..3 {
        if let Shape::Line(n) = shape {
            total += n;
            shape = Shape::Line(n * 2);
        }
    }
    total
}

fn main() {
    println!("{}", after(Shape::Line(20)));
    println!("{}", after(Shape::Line(2)));
    println!("{}", between(Shape::Line(5)));
    println!("{}", taken(Shape::Line(3)));
    println!("{}", turns(Shape::Line(1)));
}
