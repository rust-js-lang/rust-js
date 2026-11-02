//! A consumer that compiles the library's generic code from its MIR, which
//! gives a private struct's constructor as a function, `.map(Self)`, made
//! as the consumer makes its own, not imported (#476); and gives a public
//! one's itself.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let kept = dep::keep(Some(Loud(1)), Loud(2));
    println!("kept {:?}", kept.as_ref().map(|loud| loud.0));
    println!("none {}", dep::keep(None, Loud(3)).is_none());
    let wrapped: Vec<dep::Wrap> = [4, 5].into_iter().map(dep::Wrap).collect();
    println!("{wrapped:?}");
}
