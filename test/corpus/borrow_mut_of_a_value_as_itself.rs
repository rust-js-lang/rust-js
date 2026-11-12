// `x.borrow_mut()` of std's `impl<T> BorrowMut<T> for T`, a type known
// where it's called: the `&mut` to the value it's given, the value itself of
// an object and the place of a number, where it read generic code's box of
// it, `x.value`, and threw (#1487). `borrow_mut_of_a_vec_as_a_slice` is its
// `BorrowMut<[T]>`.
use std::borrow::BorrowMut;
use std::collections::HashMap;

#[derive(Debug, Default)]
struct Point {
    x: i32,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Level {
    Low,
    High(u8),
}

struct Holder {
    items: Vec<i32>,
    count: u32,
    name: String,
}

fn pushed(v: &mut Vec<i32>) -> usize {
    let b: &mut Vec<i32> = v.borrow_mut();
    b.push(9);
    b.len()
}

fn bumped(n: &mut i32) -> i32 {
    let b: &mut i32 = n.borrow_mut();
    *b += 1;
    *b
}

fn main() {
    let mut m = vec![1, 2];
    let s: &mut Vec<i32> = m.borrow_mut();
    s.push(3);
    println!("{m:?} {}", pushed(&mut m));
    println!("{m:?}");

    let mut p = Point::default();
    let b: &mut Point = p.borrow_mut();
    b.x = 2;
    b.tags.push("t".into());
    println!("{p:?}");

    let mut t = (1, String::from("a"));
    let b: &mut (i32, String) = t.borrow_mut();
    b.0 = 9;
    b.1.push('b');
    println!("{t:?}");

    let mut map: HashMap<i32, i32> = HashMap::new();
    let b: &mut HashMap<i32, i32> = map.borrow_mut();
    b.insert(1, 2);
    println!("{map:?}");

    let mut h = Holder { items: vec![1], count: 0, name: String::from("h") };
    let b: &mut Vec<i32> = h.items.borrow_mut();
    b.push(2);
    let c: &mut u32 = h.count.borrow_mut();
    *c += 5;
    let n: &mut String = h.name.borrow_mut();
    n.push('!');
    println!("{:?} {} {}", h.items, h.count, h.name);

    let mut n = 1;
    let b: &mut i32 = n.borrow_mut();
    *b += 1;
    println!("{n} {}", bumped(&mut n));
    println!("{n}");

    let mut text = String::from("x");
    let b: &mut String = text.borrow_mut();
    b.push('y');
    println!("{text}");

    let mut o = Some(1);
    let b: &mut Option<i32> = o.borrow_mut();
    *b = None;
    println!("{o:?}");

    let mut l = Level::High(1);
    let b: &mut Level = l.borrow_mut();
    *b = Level::Low;
    println!("{l:?}");

    let mut v = vec![2, 1];
    let r = &mut v;
    let b: &mut Vec<i32> = r.borrow_mut();
    b.sort();
    println!("{v:?}");

    let mut w = vec![1];
    let b = <Vec<i32> as BorrowMut<Vec<i32>>>::borrow_mut(&mut w);
    b.push(2);
    let s = BorrowMut::<[i32]>::borrow_mut(&mut w);
    s[0] = 7;
    println!("{w:?}");
}
