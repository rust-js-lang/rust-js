use dep::{decremented, doubled, first_two, fused, while_small};

pub fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    println!("{:?} {:?}", doubled(v.iter().copied()), doubled(4..9u32));
    println!("{:?} {:?}", first_two(v.iter().copied()), first_two(0..10u32));
    println!("{:?} {:?}", decremented(v.iter().copied()), decremented(1..6u32));
    println!("{:?} {:?}", fused(v.iter().copied()), fused(2..5u32));
    println!("{:?} {:?}", while_small(v.iter().copied()), while_small(0..3u32));
}
