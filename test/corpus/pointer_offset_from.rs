// `p.offset_from(q)`: how many items `p` is past `q` along one array, as
// hashbrown counts a bucket's index from its table's pointer (ADR 0407).
fn index_of(items: &[u32], at: *const u32) -> isize {
    unsafe { at.offset_from(items.as_ptr()) }
}

fn main() {
    let xs = [10, 20, 30, 40];
    let p = xs.as_ptr();
    let q = unsafe { p.add(3) };
    println!("{} {} {}", unsafe { q.offset_from(p) }, unsafe { p.offset_from(q) }, unsafe { p.offset_from(p) });
    println!("{}", index_of(&xs, unsafe { p.add(2) }));
}
