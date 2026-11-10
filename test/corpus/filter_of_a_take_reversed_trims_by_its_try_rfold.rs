// A filter's step is its source's `find`, a `try_fold`, though it's made
// knowing its back, as one generic code steps from either end is: of a
// `take(n)` reversed, its `try_rfold`, which trims its back one past where
// its `next_back` does, so the items past what it takes drop from the back,
// 5 then 4, where its `next_back` drops them as one slice, 4 then 5
// (#1226).
struct Note(u32);

impl Drop for Note {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn notes() -> Vec<Note> {
    (1..=5).map(Note).collect()
}

fn newest_first<I: DoubleEndedIterator>(it: I) -> impl DoubleEndedIterator<Item = I::Item> {
    it.rev()
}

fn ends<I: DoubleEndedIterator<Item = Note>>(mut it: I) -> (Option<u32>, Option<u32>) {
    let first = it.next().map(|n| n.0);
    let last = it.next_back().map(|n| n.0);
    (first, last)
}

fn main() {
    println!("{:?}", ends(newest_first(notes().into_iter().take(3)).filter(|n| n.0 > 1)));
    println!("{:?}", ends(newest_first(notes().into_iter().take(4)).filter(|n| n.0 % 2 == 0)));
}
