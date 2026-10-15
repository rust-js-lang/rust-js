// A union is the object of the one field it holds: a field read gives what
// it holds there, or under a field of the same type, and a field written is
// the one it holds now (ADR 0406).
use std::mem::ManuallyDrop;

#[derive(Clone, Copy)]
union Number {
    int: i32,
    float: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Int,
    Float,
}

#[derive(Clone, Copy)]
struct Value {
    kind: Kind,
    number: Number,
}

impl Value {
    fn int(n: i32) -> Value {
        Value { kind: Kind::Int, number: Number { int: n } }
    }
    fn float(x: f64) -> Value {
        Value { kind: Kind::Float, number: Number { float: x } }
    }
    fn show(&self) -> String {
        unsafe {
            match self.kind {
                Kind::Int => format!("int {}", self.number.int),
                Kind::Float => format!("float {}", self.number.float),
            }
        }
    }
    fn is_zero(&self) -> bool {
        unsafe {
            match self {
                Value { kind: Kind::Int, number: Number { int: 0 } } => true,
                Value { kind: Kind::Float, number: Number { float } } => *float == 0.0,
                _ => false,
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
union Shape {
    point: Point,
    nothing: (),
}

union Same {
    left: u16,
    right: u16,
}

fn set_float(n: &mut Number, x: f64) {
    n.float = x;
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

union Holder {
    noisy: ManuallyDrop<Noisy>,
    count: u8,
}

impl Drop for Holder {
    fn drop(&mut self) {
        println!("drop holder");
    }
}

fn main() {
    let mut values = vec![Value::int(7), Value::float(2.5), Value::int(0), Value::float(0.0)];
    for v in &values {
        println!("{} {}", v.show(), v.is_zero());
    }
    // A copy is a union of its own: writing one leaves the other.
    let copy = values[0];
    values[0].number.int = 8;
    values[0].kind = Kind::Int;
    unsafe {
        println!("{} {}", copy.number.int, values[0].number.int);
    }
    // Written as another field, it holds that one now.
    values[1].kind = Kind::Int;
    values[1].number.int = -3;
    println!("{}", values[1].show());
    let mut n = Number { int: 1 };
    set_float(&mut n, 0.5);
    println!("{}", unsafe { n.float });

    // Fields of one type read each other's: the same bytes.
    let mut same = Same { left: 40 };
    unsafe {
        same.right += 2;
        println!("{} {}", same.left, same.right);
        let Same { left } = same;
        println!("{left}");
    }

    // One holding what's changed in place: its copy holds a copy.
    let mut shape = Shape { point: Point { x: 1, y: 2 } };
    let kept = shape;
    unsafe {
        shape.point.x = 10;
        println!("{} {} {}", kept.point.x, shape.point.x, shape.point.y);
        // A field of no bytes reads nothing, whatever it holds.
        println!("{:?}", shape.nothing);
    }
    let boxed = ManuallyDrop::new(5);
    println!("{}", *boxed + 1);

    // A union's own `Drop` runs; its fields are never dropped.
    {
        let held = Holder { noisy: ManuallyDrop::new(Noisy("inside")) };
        println!("{}", unsafe { held.noisy.0 });
    }
    let mut counted = Holder { count: 1 };
    unsafe {
        counted.count += 1;
        println!("{}", counted.count);
    }
}
