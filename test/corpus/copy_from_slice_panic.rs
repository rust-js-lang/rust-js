//@ run-fail: copy_from_slice: source slice length (2) does not match destination slice length (3)
// `copy_from_slice` of another length panics with std's message (ADR 0324).

fn main() {
    let mut target = [0u8; 3];
    target.copy_from_slice(&[1, 2]);
    println!("{target:?}");
}
