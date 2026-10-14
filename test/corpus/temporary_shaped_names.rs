// A local the program names as MIR names its temporaries, `_5`, keeps its
// name, `_5_`, where tidy would take `_5` for a temporary and name it as
// one, `value`.
fn main() {
    let mut _5 = vec![1, 2];
    _5.push(3);
    let total: i32 = _5.iter().sum();
    println!("{} {:?}", total, _5);
}
