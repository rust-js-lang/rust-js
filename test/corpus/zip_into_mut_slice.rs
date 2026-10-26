// `zip(b)` of a `&mut [T]` or a `&mut Vec<T>` steps `b` as its `IntoIterator`
// is, a `&mut` to each item, as `b.iter_mut()` is (ADR 0926): num-bigint's
// `for (ai, bi) in a.iter().zip(b) { *bi += *ai; }`.

fn add_into(a: &[u32], b: &mut [u32]) {
    for (ai, bi) in a.iter().zip(b) {
        *bi += *ai;
    }
}

fn add_vec(a: &[u32], b: &mut Vec<u32>) {
    for (bi, ai) in b.iter_mut().zip(a) {
        *bi *= *ai;
    }
    for (ai, bi) in a.iter().zip(b) {
        *bi += *ai;
    }
}

fn main() {
    let a = [1, 2, 3];
    let mut b = [10, 20, 30, 40];
    add_into(&a, &mut b);
    println!("{b:?}");
    let mut v = vec![5, 6];
    add_vec(&a, &mut v);
    println!("{v:?}");
    let mut w = [1, 1];
    let pairs: Vec<(&i32, &mut i32)> = [7, 8].iter().zip(&mut w).collect();
    for (x, y) in pairs {
        *y += *x;
    }
    println!("{w:?}");
    let mut c = [0u8; 3];
    a.iter().zip(&mut c).for_each(|(x, y)| *y = *x as u8 * 2);
    println!("{c:?}");
}
