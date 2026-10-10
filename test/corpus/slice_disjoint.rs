// `get_disjoint_mut` of indices or ranges: a `&mut` to each, checked as
// std checks them, in order: each in bounds, then apart from those before.

#[derive(Debug)]
struct Cell {
    n: i32,
}

fn main() {
    let mut v = vec![1, 2, 3, 4, 5, 6];
    if let Ok([a, b]) = v.get_disjoint_mut([0, 5]) {
        std::mem::swap(a, b);
        *a *= 10;
    }
    println!("{v:?}");
    println!("{:?}", v.get_disjoint_mut([1, 9]));
    println!("{:?}", v.get_disjoint_mut([2, 2]));
    println!("{:?}", v.get_disjoint_mut([9, 2, 2]).unwrap_err());
    let error = v.get_disjoint_mut([3, 3]).unwrap_err();
    println!("{}", error == std::slice::GetDisjointMutError::OverlappingIndices);

    if let Ok([left, right]) = v.get_disjoint_mut([0..2, 3..6]) {
        left.copy_from_slice(&right[1..]);
        right.reverse();
    }
    println!("{v:?}");
    println!("{:?}", v.get_disjoint_mut([0..3, 2..4]).is_err());
    println!("{:?}", v.get_disjoint_mut([0..=2, 3..=6]).is_err());
    println!("{:?}", v.get_disjoint_mut([2..=1]).is_err());
    if let Ok([a, b]) = v.get_disjoint_mut([0..=1, 2..=2]) {
        a[1] = b[0];
    }
    println!("{v:?}");
    unsafe {
        let [x, y] = v.get_disjoint_unchecked_mut([1, 4]);
        *x += *y;
    }
    println!("{v:?}");

    let mut cells = [Cell { n: 1 }, Cell { n: 2 }, Cell { n: 3 }];
    let [first, last] = cells.get_disjoint_mut([0, 2]).unwrap();
    first.n += last.n;
    last.n = 0;
    println!("{cells:?}");
}
