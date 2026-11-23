// A slice's `split_off_first_mut()` and `split_off_last_mut()` of an empty
// slice give `None` and leave it empty, as std's do: no view past its end,
// whose length would be -1 (#1979's `slice-split-off-end-empty`).

fn main() {
    let mut empty: &mut [i32] = &mut [];
    let first = empty.split_off_first_mut();
    println!("{first:?} {}", empty.len());
    let last = empty.split_off_last_mut();
    println!("{last:?} {}", empty.len());
    let mut items: &mut [i32] = &mut [1, 2, 3];
    if let Some(first) = items.split_off_first_mut() {
        *first += 10;
    }
    let last = items.split_off_last_mut().copied();
    println!("{last:?} {items:?}");
    let mut one: &mut [i32] = &mut [7];
    let only = one.split_off_last_mut().copied();
    let again = one.split_off_first_mut();
    println!("{only:?} {again:?} {}", one.len());
}
