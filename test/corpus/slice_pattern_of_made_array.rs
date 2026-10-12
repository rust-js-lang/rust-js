// A slice pattern's rest of an array made where it's taken apart, and its
// last item: the array held first, then read so.
#[derive(Debug, Clone)]
struct N(u8);

const fn middle() -> [N; 2] {
    let [_, x @ .., _] = [N(1), N(2), N(3), N(4)];
    x
}

fn main() {
    let [.., last] = [N(5), N(6), N(7)];
    let [_, rest @ ..] = [N(8), N(9), N(10)];
    println!("{:?} {:?} {:?}", middle(), last, rest);
}
