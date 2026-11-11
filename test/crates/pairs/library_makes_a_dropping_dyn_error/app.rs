//! A consumer that makes no `dyn Error` of its own, dropping the one its
//! library made of a value with a destructor (#1403).
pub fn main() {
    match dep::failing(5) {
        Ok(n) => println!("ok {n}"),
        Err(e) => println!("err {e}"),
    }
    println!("end");
}
