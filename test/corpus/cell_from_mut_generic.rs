// `Cell::from_mut` of a `&mut` generic code is given, a type parameter's
// `&mut T`, is a cell of the place the `&mut` is to, as of a concrete
// `&mut`: its `replace`, `set`, `get` and `take` read and write the
// caller's value, an object's, a number's and a text's, and `get_mut` of a
// generic `Cell<T>` gives the `&mut T` back; and `as_slice_of_cells` of a
// slice of a type parameter's items, or of numbers, is a cell of each item
// (#614).
use std::cell::Cell;
use std::mem;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
}

fn replaced<T>(r: &mut T, v: T) -> T {
    Cell::from_mut(r).replace(v)
}

fn set_then_get<T: Copy>(r: &mut T, v: T) -> T {
    let cell = Cell::from_mut(r);
    let old = cell.get();
    cell.set(v);
    old
}

fn taken<T: Default>(r: &mut T) -> T {
    Cell::from_mut(r).take()
}

fn back<T>(cell: &mut Cell<T>, v: T) -> T {
    mem::replace(cell.get_mut(), v)
}

fn rotated<T: Copy>(items: &mut [T]) {
    let cells = Cell::from_mut(items).as_slice_of_cells();
    if let [first, .., last] = cells {
        first.swap(last);
    }
}

fn swapped_first<T>(items: &mut [T], v: T) -> T {
    let cells = Cell::from_mut(items).as_slice_of_cells();
    cells[0].replace(v)
}

fn main() {
    let mut v = vec![1];
    let old = replaced(&mut v, vec![2]);
    v.push(3);
    println!("{v:?} {old:?}");

    let mut p = Point { x: 1 };
    let old = replaced(&mut p, Point { x: 2 });
    p.x += 10;
    println!("{p:?} {old:?}");

    let mut n = 5u32;
    let old = replaced(&mut n, 6);
    n += 1;
    println!("{n} {old}");

    let mut s = String::from("a");
    let old = replaced(&mut s, String::from("b"));
    s.push('!');
    println!("{s} {old}");

    let mut p = Point { x: 3 };
    let old = set_then_get(&mut p, Point { x: 4 });
    println!("{p:?} {old:?}");

    let mut n = 7i64;
    let old = set_then_get(&mut n, 8);
    println!("{n} {old}");

    let mut v = vec![4, 5];
    let old = taken(&mut v);
    v.push(6);
    println!("{v:?} {old:?}");

    let mut c = Cell::new(vec![1]);
    let old = back(&mut c, vec![9]);
    println!("{:?} {old:?}", c.into_inner());

    let mut c = Cell::new(3u8);
    let old = back(&mut c, 4);
    println!("{} {old}", c.get());

    // Concrete: the same calls, not through generic code.
    let mut v = vec![1, 2];
    let old = Cell::from_mut(&mut v).replace(vec![3]);
    v.push(4);
    println!("{v:?} {old:?}");

    let mut n = 1.5f64;
    Cell::from_mut(&mut n).set(2.5);
    println!("{n}");

    let mut items = [1u32, 2, 3];
    rotated(&mut items);
    println!("{items:?}");

    let mut points = [Point { x: 1 }, Point { x: 2 }];
    rotated(&mut points);
    points[0].x += 100;
    println!("{points:?}");

    let mut texts = vec![String::from("x"), String::from("y")];
    let old = swapped_first(&mut texts, String::from("z"));
    println!("{texts:?} {old}");

    let mut numbers = [10u16, 20];
    let cells = Cell::from_mut(&mut numbers[..]).as_slice_of_cells();
    cells[1].set(cells[0].get() + 1);
    println!("{numbers:?}");
}
