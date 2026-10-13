// A tuple matched whose parts call, `match (v.pop(), n > 3)`: each part
// that acts held first, in order, `const value = v.pop();`, and what's read
// before it with it, `v.len()` of `(v.len(), v.pop())`, read as it was
// before the call; the rest read where the tuple's parts are. A variable
// is read where its part is unless something sets it after, a guard or a
// closure called in one: the tuple has it as it was.
use std::collections::HashMap;

enum Shape {
    Circle(u32),
    Square(u32),
}

fn popped(v: &mut Vec<u32>, n: u32) -> u32 {
    match (v.pop(), n > 3) {
        (Some(x), true) => x,
        (Some(_), false) => 1,
        (None, _) => 0,
    }
}

fn counted(v: &mut Vec<u32>) -> u32 {
    match (v.len(), v.pop()) {
        (n, Some(x)) => n as u32 * 10 + x,
        (n, None) => n as u32,
    }
}

fn peeked(map: &HashMap<String, Shape>, names: &mut std::iter::Peekable<std::vec::IntoIter<String>>) -> Option<u32> {
    let name = names.next()?;
    match (map.get(&name)?, names.peek().is_some()) {
        (Shape::Circle(r), true) => Some(r * 2),
        (Shape::Square(s), false) => Some(*s),
        _ => None,
    }
}

fn guarded(v: &mut Vec<u32>) -> u32 {
    let mut x = 1;
    match (v.pop(), x) {
        (_, _) if {
            x += 10;
            false
        } =>
        {
            0
        }
        (Some(b), a) => a * 100 + b + x,
        (None, a) => a + x,
    }
}

fn bumped(v: &mut Vec<u32>) -> u32 {
    let mut x = 1;
    match (v.pop(), x) {
        (_, _) if (|| {
            x += 10;
            false
        })() =>
        {
            0
        }
        (Some(b), a) => a * 100 + b + x,
        (None, a) => a + x,
    }
}

fn main() {
    let mut v = vec![4, 5];
    println!("{} {} {}", popped(&mut v, 5), popped(&mut v, 1), popped(&mut v, 9));
    let mut w = vec![7, 8];
    println!("{} {} {}", counted(&mut w), counted(&mut w), counted(&mut w));
    let mut k = vec![3];
    println!("{} {} {}", guarded(&mut k), guarded(&mut k), bumped(&mut vec![4]));
    let mut map = HashMap::new();
    map.insert("a".to_string(), Shape::Circle(3));
    map.insert("b".to_string(), Shape::Square(4));
    let mut names = vec!["a".to_string(), "b".to_string(), "a".to_string()].into_iter().peekable();
    println!("{:?} {:?} {:?} {:?}", peeked(&map, &mut names), peeked(&map, &mut names), peeked(&map, &mut names), peeked(&map, &mut names));
}
