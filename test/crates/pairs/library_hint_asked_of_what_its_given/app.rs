use dep::hint;

pub fn main() {
    let v = vec![1u32, 0, 2, 5];
    println!("{:?} {:?}", hint(v.iter().filter(|x| **x > 0)), hint(v.iter().take(2)));
    println!("{:?} {:?}", hint(v.iter().map(|x| x + 1)), hint(0..4u32));
}
