//! A trait whose impl for a struct holding a function calls it by
//! `FnOnce`, and generic code that runs it (#2609).
pub trait Run {
    fn run(self) -> u32;
}

pub struct Once<F>(pub F);

impl<F: FnOnce() -> u32> Run for Once<F> {
    fn run(self) -> u32 {
        (self.0)()
    }
}

pub fn run_it<R: Run>(r: R) -> u32 {
    r.run()
}
