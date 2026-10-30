//@ native: wasm32
//@ library-differs
// What `hash` writes, through std's `DefaultHasher` (ADR 0463): std's types
// as std's impls write them, a `usize` as a 32-bit target's, a derived
// `Hash`, a type's own, generic code's of a `T: Hash`, and a type's own
// `hash_slice`. `finish()` is std's, the same SipHash-1-3 of zero keys.

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroU32;
use std::rc::Rc;

#[derive(Hash, Clone)]
struct Point {
    x: i16,
    y: i16,
}

#[derive(Hash)]
enum Shape {
    Dot,
    Line(u8),
    Poly { sides: Vec<u16> },
}

#[derive(Hash)]
struct Labeled<T>(T, Vec<T>);

#[derive(Hash)]
#[repr(u8)]
enum Small {
    A = 3,
    B = 7,
}

#[derive(Hash)]
#[repr(i64)]
enum Wide {
    Low = -5,
    High(u8) = 1 << 40,
}

// Hashes its name as `==` would compare it: case aside.
struct Name(String);

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_lowercase().hash(state);
    }
}

// A slice of them hashes as one count, as its own `hash_slice` says.
struct Token;

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        1u8.hash(state);
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H) {
        data.len().hash(state);
    }
}

// A std hasher's own writes and a std type's, where both are known.
fn mix(hasher: &mut DefaultHasher, n: u8) {
    hasher.write_u8(n);
    n.hash(hasher);
}

fn digest<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    println!("{} {} {}", digest(&7u32), digest(&-7i64), digest(&usize::MAX));
    println!("{} {}", digest("text"), digest(&String::from("text")));
    println!("{}", digest(&(3u8, 'é', true, ())));
    println!("{} {}", digest(&Some(-4i64)), digest(&None::<i64>));
    println!("{}", digest(&Some(Some(()))));
    println!("{} {}", digest(&vec![1u16, 2, 3]), digest(&[1i8, -1]));
    println!("{} {}", digest(&['a', 'b']), digest(&VecDeque::from([1u32, 2])));
    println!("{}", digest("bytes".as_bytes()));
    let ok: Result<u32, String> = Ok(1);
    let err: Result<u32, String> = Err("e".into());
    println!("{} {}", digest(&ok), digest(&err));
    println!("{}", digest(&(Box::new(5u32), Rc::new("rc"), &mut 6u32)));
    println!("{} {}", digest(&NonZeroU32::new(9)), digest(&3.cmp(&4)));

    let p = Point { x: -1, y: 2 };
    println!("{}", digest(&p));
    for shape in [Shape::Dot, Shape::Line(9), Shape::Poly { sides: vec![4, 5] }] {
        println!("{}", digest(&shape));
    }
    println!("{}", digest(&vec![Some(p.clone()), None]));
    println!("{}", digest(&Labeled(p.clone(), vec![p, Point { x: 3, y: 4 }])));
    println!("{}", digest(&Labeled("a".to_string(), vec!["b".to_string()])));
    println!("{} {}", digest(&Small::A), digest(&Small::B));
    println!("{} {}", digest(&Wide::Low), digest(&Wide::High(2)));
    println!("{}", digest(&Name("MiXed".into())) == digest(&Name("mixed".into())));
    println!("{} {}", digest(&[Token, Token][..]), digest(&[[Token], [Token]]));

    let mut hasher = DefaultHasher::default();
    7u32.hash(&mut hasher);
    u32::hash_slice(&[1, 2], &mut hasher);
    Point::hash_slice(&[Point { x: 0, y: 0 }], &mut hasher);
    mix(&mut hasher, 4);
    let before = hasher.clone();
    hasher.write_u8(1);
    let first = hasher.finish();
    hasher.write_u8(2);
    println!("{first} {} {}", hasher.finish(), before.finish());
}
