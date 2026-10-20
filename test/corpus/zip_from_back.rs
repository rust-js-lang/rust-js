// `rev()` of a zip of two arrays' iterators: the longer trimmed from its
// back to the shorter's length, then each's back paired, as Rust's
// `Zip::next_back` does, `$zipBack(a, b)`, of `iter()`'s and `iter_mut()`'s
// alike (ADR 0493).
fn main() {
    let mut a = vec![1, 2, 3];
    let mut b = vec![10, 20];
    for (x, y) in a.iter_mut().zip(b.iter_mut()).rev() {
        *x += *y;
        *y = 0;
    }
    println!("{a:?} {b:?}");

    let names = ["p", "q", "r"];
    let nums = [1, 2, 3, 4];
    let back: Vec<(&&str, &i32)> = names.iter().zip(nums.iter()).rev().collect();
    println!("{back:?}");
    let shorter: Vec<(&i32, &&str)> = nums[..2].iter().zip(names.iter()).rev().collect();
    println!("{shorter:?}");

    let mut words = vec![String::from("x"), String::from("y")];
    let mut counts = vec![1, 2, 3];
    for (w, c) in words.iter_mut().zip(counts.iter_mut()).rev() {
        w.push('!');
        *c *= 2;
        if *c == 4 {
            break;
        }
    }
    println!("{words:?} {counts:?}");

    let none: Vec<(&i32, &i32)> = [1, 2].iter().zip([].iter()).rev().collect();
    println!("{none:?}");
}
