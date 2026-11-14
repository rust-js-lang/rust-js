//@ edition: 2018
//! itertools' `coalesce` and `dedup`, as its 0.14 writes them, of its
//! edition, 2018's, whose closure captures the `&mut` itself: `next`
//! writes the item that ended a run through it, `*last = Some(Some(next_))`,
//! whose old value's drop may unwind, so its cleanup writes the new one too,
//! through the `&mut` it reads again there (#1681).
pub struct CoalesceBy<I, F, C>
where
    I: Iterator,
    C: CountItem<I::Item>,
{
    iter: I,
    last: Option<Option<C::CItem>>,
    f: F,
}

pub trait CoalescePredicate<Item, T> {
    fn coalesce_pair(&mut self, t: T, item: Item) -> Result<T, (T, T)>;
}

impl<I, F, C> Iterator for CoalesceBy<I, F, C>
where
    I: Iterator,
    F: CoalescePredicate<I::Item, C::CItem>,
    C: CountItem<I::Item>,
{
    type Item = C::CItem;

    fn next(&mut self) -> Option<Self::Item> {
        let Self { iter, last, f } = self;
        let init = match last {
            Some(elt) => elt.take(),
            None => {
                *last = Some(None);
                iter.next().map(C::new)
            }
        }?;

        Some(
            iter.try_fold(init, |accum, next| match f.coalesce_pair(accum, next) {
                Ok(joined) => Ok(joined),
                Err((last_, next_)) => {
                    *last = Some(Some(next_));
                    Err(last_)
                }
            })
            .unwrap_or_else(|x| x),
        )
    }
}

pub struct NoCount;

pub trait CountItem<T> {
    type CItem;
    fn new(t: T) -> Self::CItem;
}

impl<T> CountItem<T> for NoCount {
    type CItem = T;
    fn new(t: T) -> T {
        t
    }
}

impl<F, Item, T> CoalescePredicate<Item, T> for F
where
    F: FnMut(T, Item) -> Result<T, (T, T)>,
{
    fn coalesce_pair(&mut self, t: T, item: Item) -> Result<T, (T, T)> {
        self(t, item)
    }
}

pub fn coalesce<I, F>(iter: I, f: F) -> CoalesceBy<I, F, NoCount>
where
    I: Iterator,
{
    CoalesceBy {
        last: None,
        iter,
        f,
    }
}

pub struct DedupPred2CoalescePred<DP>(DP);

pub trait DedupPredicate<T> {
    fn dedup_pair(&mut self, a: &T, b: &T) -> bool;
}

impl<DP, T> CoalescePredicate<T, T> for DedupPred2CoalescePred<DP>
where
    DP: DedupPredicate<T>,
{
    fn coalesce_pair(&mut self, t: T, item: T) -> Result<T, (T, T)> {
        if self.0.dedup_pair(&t, &item) {
            Ok(t)
        } else {
            Err((t, item))
        }
    }
}

pub struct DedupEq;

impl<T: PartialEq> DedupPredicate<T> for DedupEq {
    fn dedup_pair(&mut self, a: &T, b: &T) -> bool {
        a == b
    }
}

pub fn dedup<I>(iter: I) -> CoalesceBy<I, DedupPred2CoalescePred<DedupEq>, NoCount>
where
    I: Iterator,
{
    CoalesceBy {
        last: None,
        iter,
        f: DedupPred2CoalescePred(DedupEq),
    }
}
