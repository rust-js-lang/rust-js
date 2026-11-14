//! An adapter that folds an iterator's items into runs, as itertools'
//! `coalesce` does, and keeps the item that ended the last run, written
//! through a `&mut` its closure captures over what an earlier run left: the
//! write drops what it replaces, and where that drop panics, the new value
//! is still written as the panic unwinds, and the run so far dropped after.
pub struct Runs<I: Iterator> {
    iter: I,
    last: Option<I::Item>,
}

impl<I: Iterator> Runs<I> {
    pub fn new(iter: I) -> Self {
        Runs { iter, last: None }
    }

    pub fn next_run<F>(&mut self, mut join: F) -> Option<I::Item>
    where
        F: FnMut(I::Item, I::Item) -> Result<I::Item, (I::Item, I::Item)>,
    {
        let Self { iter, last } = self;
        let init = iter.next()?;
        Some(
            iter.try_fold(init, |accum, next| match join(accum, next) {
                Ok(joined) => Ok(joined),
                Err((done, kept)) => {
                    *last = Some(kept);
                    Err(done)
                }
            })
            .unwrap_or_else(|x| x),
        )
    }

    pub fn last(&self) -> Option<&I::Item> {
        self.last.as_ref()
    }
}
