// unbounded_shl and unbounded_shr, past the width 0 or a negative's -1;
// next_multiple_of and checked_next_multiple_of; wrapping_rem_euclid and
// wrapping_div_euclid, of numbers and BigInts.
fn id<T>(x: T) -> T { x }
fn main() {
    println!("{} {} {}", id(1u32).unbounded_shl(40), id(1u32).unbounded_shl(31), id(3u8).unbounded_shl(7));
    println!("{} {} {}", id(8u64).unbounded_shr(70), id(-8i32).unbounded_shr(99), id(-8i32).unbounded_shr(2));
    println!("{} {} {}", id(-1i8).unbounded_shl(8), id(i128::MIN).unbounded_shr(127), id(5i64).unbounded_shr(64));
    println!("{:?} {:?} {:?}", id(200u8).checked_next_multiple_of(16), id(7u32).checked_next_multiple_of(0), id(u64::MAX - 1).checked_next_multiple_of(10));
    println!("{:?} {:?}", id(16u16).checked_next_multiple_of(8), id(u128::MAX).checked_next_multiple_of(1));
    println!("{} {} {}", id(i32::MIN).wrapping_rem_euclid(-1), id(-7i32).wrapping_rem_euclid(3), id(i64::MIN).wrapping_div_euclid(-1));
    println!("{} {}", id(-7i128).wrapping_div_euclid(2), id(7u8).wrapping_rem_euclid(3));
    println!("{} {} {}", id(7u32).next_multiple_of(4), id(16u8).next_multiple_of(8), id(251u8).next_multiple_of(10));
    println!("{}", id(u64::MAX - 3).next_multiple_of(7));
}
