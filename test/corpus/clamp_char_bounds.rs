//@ run-fail: min > max. min = 'k', max = 'a'
// A char's clamp() of bounds the wrong way round panics showing them, as
// std's char does.
fn main() {
    let x = std::hint::black_box('z');
    println!("{}", x.clamp('k', 'a'));
}
