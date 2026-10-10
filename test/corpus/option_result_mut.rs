// `iter_mut()` of an `Option` and a `Result`, `for x in &mut option`, and a
// `Result`'s `as_mut()`: a `&mut` to what's in it, written through.

#[derive(Debug)]
struct Point {
    x: i32,
}

fn main() {
    let mut count: Option<i32> = Some(1);
    for c in count.iter_mut() {
        *c += 1;
    }
    for c in &mut count {
        *c *= 10;
    }
    let mut none: Option<i32> = None;
    for c in &mut none {
        *c = 7;
    }
    for c in none.iter_mut() {
        *c = 5;
    }
    println!("{count:?} {none:?}");
    let mut name = Some(String::from("ann"));
    if let Some(n) = name.iter_mut().next() {
        n.push('!');
    }
    let mut point = Some(Point { x: 1 });
    for p in point.iter_mut() {
        p.x += 1;
    }
    println!("{name:?} {point:?}");

    let mut ok: Result<i32, String> = Ok(2);
    if let Ok(x) = ok.as_mut() {
        *x *= 5;
    }
    for x in ok.iter_mut() {
        *x += 1;
    }
    let mut err: Result<i32, String> = Err(String::from("bad"));
    if let Err(e) = err.as_mut() {
        e.push('!');
    }
    println!("{ok:?} {err:?} {}", err.iter_mut().count());
    let mut mixed: Result<i32, Point> = Err(Point { x: 1 });
    if let Err(p) = mixed.as_mut() {
        p.x = 9;
    }
    let mut number: Result<i32, Point> = Ok(3);
    if let Ok(n) = number.as_mut() {
        *n += 1;
    }
    println!("{mixed:?} {number:?}");
}
