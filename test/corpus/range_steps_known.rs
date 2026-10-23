// A range made where it's stepped knows its numbers without stepping
// them, as std's does: `step_by`, `skip`, `count` and `last` of one are
// its bounds' arithmetic, however far apart they are; of one reversed,
// `step_by` counts down from its end.
fn main() {
    for i in (0..10).step_by(3) {
        print!("{i} ");
    }
    println!();
    let big: Vec<u32> = (0..u32::MAX).step_by(1 << 30).collect();
    println!("{big:?} {:?}", (0..u32::MAX).skip(4_000_000_000).next());
    println!("{} {} {}", (5..20).count(), (0..u32::MAX).count(), (7..3).count());
    println!("{:?} {:?} {:?}", (3..9).last(), (3..3).last(), (0..10).step_by(4).last());
    println!("{:?} {:?} {}", (0..u32::MAX).last(), (0..u32::MAX).step_by(1 << 20).last(), (0..u32::MAX).step_by(1 << 20).count());
    println!("{:?} {}", (0..100).skip(2).step_by(5).nth(3), (1..=10).step_by(3).count());
    println!("{:?}", (0..u32::MAX).rev().step_by(1 << 30).collect::<Vec<_>>());
    println!("{:?}", (0..=u32::MAX).rev().step_by(1 << 30).collect::<Vec<_>>());
    println!("{:?}", (0..u64::MAX).rev().step_by(1 << 31).take(3).collect::<Vec<_>>());
    println!("{:?} {:?}", (1..=10).rev().step_by(3).collect::<Vec<_>>(), (1..10).rev().step_by(3).collect::<Vec<_>>());
    println!("{:?} {:?}", (5..5).rev().step_by(2).collect::<Vec<_>>(), (-5i32..=5).rev().step_by(4).collect::<Vec<_>>());
}
