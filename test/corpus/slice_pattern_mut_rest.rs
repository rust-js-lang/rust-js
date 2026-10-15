// A slice pattern's rest through a `&mut`, `rest @ ..`, is a view of those
// items, as `&mut xs[1..]` is: what's written through it is the slice's.
// An item counted from the end, `last`, is its place, `v[v.length - 1]`
// (ADR 0389).
fn bump(xs: &mut [i32]) {
    for x in xs.iter_mut() {
        *x += 1;
    }
}

fn zero(x: &mut i32) {
    *x = 0;
}

fn ends(v: &mut Vec<i32>) {
    if let [first, middle @ .., last] = &mut v[..] {
        std::mem::swap(first, last);
        bump(middle);
    }
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5];
    if let [head, rest @ ..] = &mut v[..] {
        *head = 10;
        rest.sort_by(|a, b| b.cmp(a));
    }
    println!("{v:?}");
    ends(&mut v);
    println!("{v:?}");
    if let [.., last] = &mut v[..] {
        zero(last);
    }
    println!("{v:?}");
    let mut a = [1, 2, 3];
    if let [first, rest @ ..] = &mut a {
        *first += 1;
        rest[0] = 9;
    }
    println!("{a:?}");
    let mut grid = [1, 2, 3, 4];
    let [_, ref mut inner @ .., _] = grid;
    inner.reverse();
    println!("{grid:?}");
}
