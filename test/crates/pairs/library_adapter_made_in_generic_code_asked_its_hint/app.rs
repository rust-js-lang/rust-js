use dep::{doubled, first_two};

pub fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    println!("{:?} {:?}", doubled(v.iter().copied()), doubled(4..9u32));
    println!("{:?} {:?}", first_two(v.iter().copied()), first_two(0..10u32));
}
