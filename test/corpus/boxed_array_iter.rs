// 1.99's `IntoIterator` of a `Box<[T; N]>`: by value its items, by `&` and
// `&mut` places of them, where 1.98 iterated the array through a `&`.

#[derive(Debug)]
struct Point {
    x: i32,
}

fn main() {
    let names = Box::new([String::from("a"), String::from("b")]);
    let owned: Vec<String> = names.into_iter().map(|s| s + "!").collect();
    println!("{owned:?}");

    let mut counts = Box::new([1, 2, 3]);
    for n in &mut counts {
        *n *= 10;
    }
    let total: i32 = (&counts).into_iter().sum();
    println!("{counts:?} {total}");

    let mut points = Box::new([Point { x: 1 }, Point { x: 2 }]);
    for p in &mut points {
        p.x += 1;
    }
    for p in (&mut points).into_iter().rev() {
        p.x *= 2;
    }
    println!("{points:?}");

    let mut seen = Vec::new();
    for n in counts {
        seen.push(n + 1);
    }
    println!("{seen:?} {:?}", Box::new([1u8, 2]).into_iter().rev().collect::<Vec<_>>());
}
