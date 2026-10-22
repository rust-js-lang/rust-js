// A grid's rows changed only through `as_flattened_mut()` are each an
// array of their own: `[[0; 2]; 3]` isn't one row three times.
fn main() {
    let mut grid = [[0u8; 2]; 3];
    grid.as_flattened_mut()[3] = 9;
    println!("{grid:?}");

    let mut wide = vec![[0u16; 2]; 2];
    for x in wide.as_flattened_mut() {
        *x += 1;
        break;
    }
    println!("{wide:?}");
}
