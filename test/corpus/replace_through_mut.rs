// `*r = v` through a `&mut` replaces the whole value, as Rust does. A `&mut`
// to an object is the object, so it becomes `v` in place, and every name
// for it sees it; an enum mixing variants with fields and without, whose
// `Off` is a string, is reached through its place, as a number is.
use std::mem;

#[derive(Debug, Default, Clone, PartialEq)]
struct Counter {
    n: u32,
    label: String,
    items: Vec<u32>,
}

impl Counter {
    fn reset(&mut self) {
        *self = Counter::default();
    }
    fn restart(&mut self, n: u32) {
        *self = Counter { n, label: format!("from {}", self.n), ..Default::default() };
        self.items.push(n);
    }
}

fn replace_with(c: &mut Counter, n: u32) -> Counter {
    mem::replace(c, Counter { n, label: "new".into(), items: vec![n] })
}

fn swap_two(a: &mut Counter, b: &mut Counter) {
    mem::swap(a, b)
}

fn take_all(v: &mut Vec<u32>) -> Vec<u32> {
    mem::take(v)
}

#[derive(Debug, Clone, Copy, Default)]
struct P {
    x: i32,
    y: i32,
}

fn origin(p: &mut P) {
    *p = P::default();
}

// A `ref mut` binding of an object, replaced whole.
fn through_ref_mut() -> i32 {
    let mut a = P { x: 1, y: 0 };
    let cur = &mut a;
    match *cur {
        ref mut n => *n = P { x: 2, y: 3 },
    }
    a.x + a.y
}

// The same, its `&mut` in a `let mut`: still `a`, replaced whole.
#[allow(unused_mut)]
fn through_ref_mut_variable() -> i32 {
    let mut a = P { x: 1, y: 0 };
    let mut cur = &mut a;
    match *cur {
        ref mut n => *n = P { x: 4, y: 5 },
    }
    a.x + a.y
}

#[derive(Debug, Clone, Copy)]
struct Q {
    x: i32,
}

fn swap_points(a: &mut Q, b: &mut Q) {
    mem::swap(a, b)
}

#[derive(Debug, PartialEq)]
enum Light {
    Off,
    On { level: u8 },
}

impl Light {
    fn toggle(&mut self) {
        *self = match self {
            Light::Off => Light::On { level: 5 },
            Light::On { .. } => Light::Off,
        };
    }
}

fn set(l: &mut Light, level: u8) {
    *l = Light::On { level };
}

fn brighter(l: &mut Light) {
    if let Light::On { level } = l {
        *level += 1;
    }
}

#[derive(Debug, PartialEq)]
enum Shape {
    Circle { r: f64 },
    Square { side: f64 },
}

fn flip(s: &mut Shape) {
    *s = match s {
        Shape::Circle { r } => Shape::Square { side: *r * 2.0 },
        Shape::Square { side } => Shape::Circle { r: *side },
    };
}

// A string replaced whole is its place's: it's still cloned as itself.
fn shout(s: &mut String) {
    *s = format!("{s}!");
}

fn refill(v: &mut Vec<u32>) {
    *v = vec![9, 8];
}

fn zero(t: &mut (u32, String)) {
    *t = (0, "zero".into());
}

fn main() {
    let mut c = Counter { n: 3, label: "a".into(), items: vec![1, 2] };
    let kept = c.clone();
    c.reset();
    println!("{c:?} {kept:?}");
    c.restart(4);
    println!("{c:?}");
    let old = replace_with(&mut c, 7);
    println!("{old:?} {c:?}");
    let mut d = Counter { n: 1, ..Default::default() };
    swap_two(&mut c, &mut d);
    println!("{c:?} {d:?}");
    let mut row = vec![Counter::default(), Counter { n: 2, ..Default::default() }];
    row[1].restart(6);
    println!("{row:?}");
    let mut items = vec![4, 5];
    println!("{:?} {items:?}", take_all(&mut items));

    let a = P { x: 1, y: 2 };
    let mut b = a;
    origin(&mut b);
    println!("{a:?} {b:?}");
    println!("{}", through_ref_mut());
    println!("{}", through_ref_mut_variable());
    let first = Q { x: 3 };
    let mut second = first;
    let mut third = Q { x: 5 };
    swap_points(&mut second, &mut third);
    println!("{first:?} {second:?} {third:?}");

    let mut l = Light::Off;
    l.toggle();
    brighter(&mut l);
    println!("{l:?}");
    l.toggle();
    println!("{l:?}");
    set(&mut l, 9);
    println!("{l:?} {}", l == Light::On { level: 9 });
    let mut lights = vec![Light::Off, Light::On { level: 1 }];
    for light in lights.iter_mut() {
        light.toggle();
    }
    brighter(&mut lights[0]);
    println!("{lights:?}");

    let mut s = Shape::Circle { r: 1.5 };
    flip(&mut s);
    println!("{s:?} {}", s == Shape::Square { side: 3.0 });
    flip(&mut s);
    println!("{s:?} {}", s == Shape::Circle { r: 3.0 });
    let mut v = vec![1, 2, 3];
    refill(&mut v);
    println!("{v:?}");
    let mut name = "pen".to_string();
    let copy = name.clone();
    shout(&mut name);
    println!("{name} {copy}");
    let mut t = (5, "five".to_string());
    zero(&mut t);
    println!("{t:?}");
}
