//@ library-differs
// A closure that writes to the `Formatter` its writer captured, `|disp|
// disp.fmt(f)`, called there or given to another as a `&mut dyn FnMut`, in
// a closure that writes to it too, as itertools' `FormatWith` does: what it
// writes is its writer's, the `Formatter`'s options its own (ADR 0539).
use std::cell::Cell;
use std::fmt;

struct FormatWith<'a, I, F> {
    sep: &'a str,
    inner: Cell<Option<(I, F)>>,
}

impl<I, F> fmt::Display for FormatWith<'_, I, F>
where
    I: Iterator,
    F: FnMut(I::Item, &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (mut iter, mut format) = match self.inner.take() {
            Some(t) => t,
            None => panic!("FormatWith: was already formatted once"),
        };
        if let Some(fst) = iter.next() {
            format(fst, &mut |disp: &dyn fmt::Display| disp.fmt(f))?;
            iter.try_for_each(|elt| {
                if !self.sep.is_empty() {
                    f.write_str(self.sep)?;
                }
                format(elt, &mut |disp: &dyn fmt::Display| disp.fmt(f))
            })?;
        }
        Ok(())
    }
}

struct Each<'a>(&'a [u32]);

impl fmt::Display for Each<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut show = |x: &dyn fmt::Display| x.fmt(f);
        for x in self.0 {
            show(x)?;
            show(&"|")?;
        }
        Ok(())
    }
}

fn main() {
    let pairs = FormatWith {
        sep: ", ",
        inner: Cell::new(Some((vec![(1, 'a'), (2, 'b')].into_iter(), |(n, c): (u32, char), f: &mut dyn FnMut(&dyn fmt::Display) -> fmt::Result| {
            f(&n)?;
            f(&"=")?;
            f(&c)
        }))),
    };
    println!("{pairs}");
    println!("{} [{:>3}]", Each(&[1, 2]), Each(&[7]));
}
