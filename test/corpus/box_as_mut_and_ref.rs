// A `Box`'s `as_mut()` and `as_ref()`, `AsMut` and `AsRef` of what it
// holds: a `&mut` to it and a `&` to it, as `&mut *b` and `&*b` are, of a
// `Vec`, a number, a struct, and in generic code given a `Box` (#116).
#[derive(Debug)]
struct Point {
    x: i32,
}

fn grow<T: AsMut<Vec<i32>>>(mut v: T) -> T {
    v.as_mut().push(9);
    v
}

fn shown<T: AsRef<str>>(s: T) -> usize {
    s.as_ref().len()
}

fn main() {
    let mut b = Box::new(vec![1]);
    b.as_mut().push(2);
    let r: &Vec<i32> = b.as_ref();
    println!("{r:?}");
    let mut n = Box::new(5);
    *n.as_mut() += 1;
    let m: &mut i32 = n.as_mut();
    *m *= 10;
    println!("{} {}", n, n.as_ref() + 1);
    let mut p = Box::new(Point { x: 1 });
    p.as_mut().x = 7;
    println!("{:?} {}", p.as_ref(), p.as_ref().x);
    let g = grow(Box::new(vec![3]));
    println!("{g:?} {}", shown(Box::<str>::from("héllo")));
}
