// A range made where it's stepped knows its numbers without stepping
// them, as std's does: `step_by`, `skip`, `count` and `last` of one are
// its bounds' arithmetic, however far apart they are.
fn main() {
    for i in (0..10).step_by(3) {
        print!("{i} ");
    }
    println!();
    let big: Vec<u32> = (0..u32::MAX).step_by(1 << 30).collect();
    println!("{big:?} {:?}", (0..u32::MAX).skip(4_000_000_000).next());
    println!("{} {} {}", (5..20).count(), (0..u32::MAX).count(), (7..3).count());
    println!("{:?} {:?} {:?}", (3..9).last(), (3..3).last(), (0..10).step_by(4).last());
    println!("{:?}", (0..u32::MAX).last());
    println!("{:?} {}", (0..100).skip(2).step_by(5).nth(3), (1..=10).step_by(3).count());
}
