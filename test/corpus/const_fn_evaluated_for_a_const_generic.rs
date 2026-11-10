// A `const fn` rustc evaluates for a const generic's argument, as
// zerocopy's `hash_name` is for `HasField<.., { ident_id!(value) }>`, and
// calls at run time too: its body is the evaluator's, whose locals' kinds
// rustc cleared, and asking one if it's a variable the program named
// panicked rust-js (#1281).
const fn sum_to(n: u32) -> u32 {
    let mut total: u32 = 0;
    let mut i = 0;
    while i < n {
        total = total.wrapping_add(i);
        i += 1;
    }
    total
}

struct Width<const N: u32>;

fn main() {
    let _: Width<{ sum_to(4) }> = Width;
    println!("{}", sum_to(5));
}
