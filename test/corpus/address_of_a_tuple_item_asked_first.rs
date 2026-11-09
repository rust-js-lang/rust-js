//@ run-pass
// A tuple's field's address asked before its container's, where the
// tuple is no type a record says where it's stored (#1113): a tuple struct's
// first field, a tuple, and an array's first item, a tuple, are at their
// container's address, as their fields are (#1019, #1081).
struct Last((u8, u32), u16);

fn main() {
    let last = Last((1, 2), 3);
    let field = &raw const (last.0).1 as usize;
    let start = &raw const last.0 as usize;
    println!("tuple struct {} {}", field - start, last.1);

    let held = [(4u8, 5u32), (6, 7)];
    let item = &raw const held[0].1 as usize;
    let array = &raw const held as usize;
    println!("array {}", item - array);
}
