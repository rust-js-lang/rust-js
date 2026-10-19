// An impl for `&mut Money`, rust_decimal's `impl AddAssign for &mut Decimal`:
// its `&mut self`, a `&mut` to a `&mut` to an object, a cell of the object,
// called where its type is known and through generic code's dictionary.

use std::ops::AddAssign;

#[derive(Debug, Clone, Copy)]
struct Money {
    cents: i64,
    note: [u8; 2],
}

impl AddAssign for Money {
    fn add_assign(&mut self, other: Money) {
        self.cents += other.cents;
    }
}

impl AddAssign<Money> for &mut Money {
    fn add_assign(&mut self, other: Money) {
        Money::add_assign(*self, other)
    }
}

fn bump<T: AddAssign<Money>>(mut total: T, by: Money) -> T {
    total += by;
    total
}

fn main() {
    let mut m = Money { cents: 5, note: [1, 2] };
    let five = Money { cents: 5, note: [0, 0] };
    {
        let mut r = &mut m;
        r += five;
        let r = bump(r, five);
        r.cents += 1;
    }
    let mut other = Money {
        cents: 100,
        note: [0, 0],
    };
    let r = &mut other;
    *r += five;
    println!("{:?} {:?} {}", m, other, m.note.len());
}
