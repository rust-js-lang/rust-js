// A slice's `starts_with`, `ends_with`, `strip_prefix`, `strip_suffix` and
// `strip_circumfix` compare its items by their own `==`: a struct's derived
// one, a hand-written one, and a generic item's.

#[derive(PartialEq, Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Loose(i32);

// Equal when they're within one of each other.
impl PartialEq for Loose {
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0).abs() <= 1
    }
}

fn framed<T: PartialEq>(items: &[T], edge: &[T]) -> bool {
    items.starts_with(edge) && items.ends_with(edge)
}

fn main() {
    let path = vec![Point { x: 0, y: 0 }, Point { x: 1, y: 2 }, Point { x: 0, y: 0 }];
    let origin = [Point { x: 0, y: 0 }];
    println!("{} {}", path.starts_with(&origin), path.ends_with(&path[1..]));
    println!("{:?}", path.strip_prefix(&origin));
    println!("{:?}", path.strip_suffix(&[Point { x: 9, y: 9 }]));
    println!("{:?}", path.strip_circumfix(&origin, &origin));
    println!("{:?}", path[..1].strip_circumfix(&origin, &origin));

    let loose = [Loose(1), Loose(5), Loose(9)];
    println!("{} {}", loose.starts_with(&[Loose(2)]), loose.ends_with(&[Loose(7)]));
    println!("{:?}", loose.strip_prefix(&[Loose(0), Loose(6)]));
    println!("{} {}", framed(&path, &origin), framed(&loose, &[Loose(1)]));
    println!("{:?}", [1.5, 2.0].strip_circumfix(&[1.5], &[2.0]));
}
