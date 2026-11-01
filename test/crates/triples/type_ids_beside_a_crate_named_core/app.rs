//! `dyn Any`s of three crates, one named `core` (#455), told apart where
//! each is asked (ADR 1054).
pub fn main() {
    let some = dep::some();
    let text = dep::text();
    let own = core::own();
    println!("{} {} {}", core::which(&*some), core::which(&*text), core::which(&*own));
    println!("{} {}", core::which(&Some(2)), core::which(&String::new()));
    println!("{:?} {}", some.downcast_ref::<Option<i32>>(), own.is::<Option<i32>>());
    println!("{:?}", text.downcast_ref::<String>());
}
