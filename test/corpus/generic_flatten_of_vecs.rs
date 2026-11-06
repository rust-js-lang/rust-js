// Generic code's flatten of a slice's `Vec`s of a type parameter's items:
// a `&Vec<T>` has no constant size whatever `T` is, as std's
// `ConstSizeIntoIterator` gives one of an array alone, so no caller is
// asked it (#813). And of arrays of a caller's length, which it gives.
fn flat<T: Clone>(rows: &[Vec<T>]) -> Vec<T> {
    rows.iter().flatten().cloned().collect()
}

fn hinted<T>(rows: &[Vec<T>]) -> (usize, Option<usize>) {
    rows.iter().flatten().size_hint()
}

fn of_arrays<T: Copy, const N: usize>(rows: &[[T; N]]) -> Vec<T> {
    rows.iter().flatten().copied().collect()
}

fn hinted_arrays<T, const N: usize>(rows: &[[T; N]]) -> (usize, Option<usize>) {
    rows.iter().flatten().size_hint()
}

fn main() {
    println!("flat {}", flat(&[vec![1u8, 2], vec![3]]).len());
    println!(
        "flat {:?}",
        flat(&[vec!["a".to_string()], vec![], vec!["b".to_string()]])
    );
    println!("hint {:?}", hinted(&[vec![1u32, 2], vec![3]]));
    println!("arrays {:?}", of_arrays(&[[1u16, 2], [3, 4]]));
    println!("arrays hint {:?}", hinted_arrays(&[[1u8, 2, 3], [4, 5, 6]]));
    let mut it = [[1u8, 2], [3, 4]].iter().flatten();
    it.next();
    println!("array hint {:?}", it.size_hint());
}
