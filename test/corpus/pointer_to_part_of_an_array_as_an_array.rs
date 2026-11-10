// A pointer to an item partway through an array, as one to an array of
// the items from it, rustc's `issue-70507.rs`'s `fst.as_ptr() as *const
// [T; N]` of a slice's part: those items, read where they are, and
// written through a `*mut [T; N]` where they are too (#1218). Of the whole
// array, its first item's, as arrayvec's.
fn chunk<const N: usize>(v: &[i32], at: usize) -> &[i32; N] {
    let (_, rest) = v.split_at(at);
    let (part, _) = rest.split_at(N);
    unsafe { &*(part.as_ptr() as *const [i32; N]) }
}

fn chunks<const N: usize>(v: &[i32]) -> Vec<&[i32; N]> {
    let mut found = Vec::new();
    let mut left = v;
    while left.len() >= N {
        let (first, rest) = left.split_at(N);
        found.push(unsafe { &*(first.as_ptr() as *const [i32; N]) });
        left = rest;
    }
    found
}

fn main() {
    let v = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    println!("{:?}", chunks::<3>(&v));
    println!("{:?} {:?}", chunk::<2>(&v, 4), chunk::<10>(&v, 0));
    let mut w = [0i16; 6];
    let middle = unsafe { &mut *(w[2..].as_mut_ptr() as *mut [i16; 3]) };
    middle[0] = 7;
    middle[2] = 9;
    *middle = [middle[0] + 1, middle[1], middle[2]];
    println!("{w:?}");
    let whole = unsafe { &*(w.as_ptr() as *const [i16; 6]) };
    println!("{}", whole.iter().map(|&b| i32::from(b)).sum::<i32>());
}
