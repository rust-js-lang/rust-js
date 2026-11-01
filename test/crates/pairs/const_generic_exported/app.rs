pub fn main() {
    println!("{} {} {}", dep::sum_n([5, 6]), dep::sum_n([7; 5]), dep::sum_n([]));
    println!("{}", dep::report());
}
