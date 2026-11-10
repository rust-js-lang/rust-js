//@ native: wasm32
// The address of a view of the bytes of an array stored by value in
// another object, a tuple struct's field or an array's item, where a
// pointer to the array is taken, which records where it's stored: in its
// container's allocation, at the array's first item's address and its byte
// offset, as its items' pointers are, whichever is asked first. It was an
// allocation of the array's own, 4,093 bytes before its items' (#1262).
struct Holder([u32; 4]);

fn main() {
    let h = Holder([1, 2, 3, 4]);
    let whole = &raw const h.0;
    let p = h.0.as_ptr();
    let q = unsafe { p.cast::<u8>().add(3) };
    println!("{} {}", q as usize - p as usize, q as usize - whole as usize);
    let w = unsafe { p.add(1) };
    println!("{}", w as usize - q as usize);
    let grid = [[5u16, 6], [7, 8]];
    let row = &raw const grid[1];
    let byte = unsafe { grid[1].as_ptr().cast::<u8>().add(1) };
    let first = grid[0].as_ptr();
    println!("{} {}", byte as usize - row as usize, byte as usize - first as usize);
    println!("{} {}", h.0[0], grid[1][1]);
}
