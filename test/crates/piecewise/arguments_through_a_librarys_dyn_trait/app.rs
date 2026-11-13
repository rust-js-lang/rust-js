//! Gives the library's trait's method, through a `dyn` and directly, a
//! `format_args!` of its own.
use dep::Sink;

pub fn main() {
    let mut sinks: Vec<Box<dyn Sink>> = vec![Box::new(dep::Log(Vec::new()))];
    for (i, sink) in sinks.iter_mut().enumerate() {
        sink.put(format_args!("{i}:{}", dep::Point(i as i32, -1))).unwrap();
        println!("{}", sink.text());
    }
    let mut own = dep::Log(Vec::new());
    own.put(format_args!("{}{}", 'p', 7)).unwrap();
    println!("{}", own.text());
}
