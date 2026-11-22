pub fn main() {
    print!("partial ");
    dep::line(1);
    print!("a");
    print!("b ");
    dep::lines();
    println!("end");
    print!("left ");
    dep::line(2);
}
