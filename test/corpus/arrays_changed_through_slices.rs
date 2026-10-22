// An array changed through a `&mut [T]`, by std's slice methods,
// `iter_mut()` or a function of a slice, is changed: a copy of it made
// before is its own, and a grid's rows, `[[0; 2]; 3]`, each their own.
fn zero(items: &mut [i32]) {
    items[0] = 0;
}

fn sorted(mut items: [i32; 3]) -> [i32; 3] {
    items.sort();
    items
}

fn main() {
    let mut a = [3, 1, 2];
    let b = a;
    a.sort();
    let c = a;
    a.reverse();
    println!("{a:?} {b:?} {c:?} {:?}", sorted(b));
    let mut d = b;
    zero(&mut d);
    for x in &mut d {
        *x += 10;
    }
    println!("{b:?} {d:?}");

    let mut grid = [[0u8; 2]; 3];
    for cell in grid[0].iter_mut() {
        *cell = 1;
    }
    grid[1].fill(7);
    grid[2].swap(0, 1);
    for row in &mut grid {
        row[1] += 1;
    }
    println!("{grid:?}");
}
