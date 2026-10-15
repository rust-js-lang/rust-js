// A `&mut` to a `&mut` of a value JS can't change in place, `&mut &mut u32`:
// a cell of the cell, as arrayvec's `ScopeExitGuard` writes a length back
// through the `&mut` it holds when it's dropped.
struct ScopeExitGuard<T, Data, F>
where
    F: FnMut(&Data, &mut T),
{
    value: T,
    data: Data,
    f: F,
}

impl<T, Data, F> Drop for ScopeExitGuard<T, Data, F>
where
    F: FnMut(&Data, &mut T),
{
    fn drop(&mut self) {
        (self.f)(&self.data, &mut self.value)
    }
}

struct Counter {
    len: u32,
}

impl Counter {
    fn extend(&mut self, n: u32) {
        let len = self.len;
        let mut guard = ScopeExitGuard {
            value: &mut self.len,
            data: len,
            f: move |&len, self_len| {
                **self_len = len;
            },
        };
        for _ in 0..n {
            guard.data += 1;
        }
    }
}

fn bump(r: &mut &mut u32) {
    **r += 1;
}

fn retarget<'a>(r: &mut &'a mut u32, other: &'a mut u32) {
    *r = other;
}

/// Each `&mut` an `Option` holds, through `as_mut()`, and a `Vec`'s
/// through `iter_mut()`: a `&mut` to each, `**r` the number.
fn each(held: &mut Option<&mut u32>, all: &mut Vec<&mut u32>) {
    if let Some(r) = held.as_mut() {
        **r += 100;
    }
    for r in all.iter_mut() {
        **r *= 3;
    }
}

fn shout(text: &mut &mut String) {
    text.push('!');
    **text = text.to_uppercase();
}

fn main() {
    let mut counter = Counter { len: 2 };
    counter.extend(3);
    println!("{}", counter.len);
    let mut a = 1;
    let mut b = 10;
    {
        let mut r = &mut a;
        bump(&mut r);
        retarget(&mut r, &mut b);
        bump(&mut r);
    }
    println!("{a} {b}");
    let (mut x, mut y, mut z) = (1, 2, 3);
    {
        let mut held = Some(&mut x);
        let mut all = vec![&mut y, &mut z];
        each(&mut held, &mut all);
    }
    println!("{x} {y} {z}");
    let mut word = String::from("hey");
    shout(&mut &mut word);
    println!("{word}");
}
