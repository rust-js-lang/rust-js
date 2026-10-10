// A slice as arrays of `N`: `as_array`, `array_windows`, `as_chunks` and
// `as_rchunks`, and of arrays as one slice, `as_flattened`. Their `_mut`
// ones are views, an array of them written whole too.

fn main() {
    let v = [1, 2, 3, 4, 5, 6, 7];
    println!("{:?} {:?}", v[..3].as_array::<3>(), v.as_array::<3>());
    for [a, b] in v.array_windows() {
        print!("{} ", a * b);
    }
    println!();
    let (chunks, rest) = v.as_chunks::<3>();
    println!("{chunks:?} {rest:?}");
    let (rest, chunks) = v.as_rchunks::<3>();
    println!("{rest:?} {chunks:?}");
    let pairs = unsafe { v[..6].as_chunks_unchecked::<2>() };
    println!("{pairs:?} {:?}", pairs.as_flattened());

    let mut w = vec![1, 2, 3, 4, 5, 6, 7];
    if let Some(all) = w[..2].as_mut_array::<2>() {
        all.swap(0, 1);
    }
    let (chunks, rest) = w.as_chunks_mut::<3>();
    chunks[0][2] = 0;
    chunks[1] = [9, 9, 9];
    chunks.swap(0, 1);
    let kept = chunks[0];
    chunks[0][0] = 8;
    print!("{kept:?} ");
    rest[0] = -1;
    println!("{w:?}");
    let (rest, chunks) = w.as_rchunks_mut::<2>();
    rest[0] = 100;
    chunks.reverse();
    for chunk in chunks.iter_mut() {
        chunk.reverse();
    }
    println!("{w:?}");

    let mut grid = [[1, 2], [3, 4], [5, 6]];
    let flat = grid.as_flattened_mut();
    flat[1] = 20;
    flat[4..].fill(0);
    flat.reverse();
    println!("{grid:?} {}", grid.as_flattened().len());
}
