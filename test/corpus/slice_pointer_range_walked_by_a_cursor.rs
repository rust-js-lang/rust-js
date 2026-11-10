//@ native: wasm32
// A slice's `as_ptr_range()` and `as_mut_ptr_range()`, the range of its
// first item's pointer to one past its last, `{ start: $ptr(items, 0), end:
// $ptr(items, items.length) }`, as `start..end` is made: measured, asked
// whether it holds an item's pointer, walked by a cursor that holds its two
// ends, as hashbrown's and smallvec's do, of a part of a `Vec` too, and of
// no items, whose ends are one; of a call's slice, called once (#1256).
struct Cursor<T> {
    at: *mut T,
    end: *mut T,
}

impl<T: Copy> Cursor<T> {
    fn over(items: &mut [T]) -> Self {
        let range = items.as_mut_ptr_range();
        Cursor {
            at: range.start,
            end: range.end,
        }
    }

    fn next_with(&mut self, f: impl Fn(T) -> T) -> Option<T> {
        if self.at == self.end {
            return None;
        }
        let old = unsafe { *self.at };
        unsafe {
            *self.at = f(old);
            self.at = self.at.add(1);
        }
        Some(old)
    }

    fn left(&self) -> usize {
        unsafe { self.end.offset_from(self.at) as usize }
    }
}

/// Its items, said once each time it's called.
fn lent(items: &mut [i64]) -> &mut [i64] {
    println!("lent {}", items.len());
    &mut items[1..]
}

fn main() {
    let a = [1u32, 2, 3, 4];
    let r = a.as_ptr_range();
    println!(
        "{} {}",
        unsafe { r.end.offset_from(r.start) },
        r.contains(&a[2..].as_ptr())
    );
    println!("{}", unsafe { *r.end.sub(1) });
    let mut v = vec![5i64, -6, 7, 8];
    let mut cursor = Cursor::over(&mut v[1..]);
    let left = cursor.left();
    let first = cursor.next_with(|x| x * 10);
    let second = cursor.next_with(|x| x * 10);
    println!("{left} {first:?} {second:?} {}", cursor.left());
    while cursor.next_with(|x| x + 1).is_some() {}
    println!("{:?} {}", cursor.next_with(|x| x), cursor.left());
    println!("{v:?}");
    let r = lent(&mut v).as_mut_ptr_range();
    println!("{}", unsafe { r.end.offset_from(r.start) });
    let empty: [u8; 0] = [];
    let r = empty.as_ptr_range();
    println!("{} {}", r.start == r.end, r.is_empty());
}
