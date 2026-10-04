// A type's own `AsMut` and `BorrowMut`, as smallvec's and arrayvec's are: a
// `&mut` to the slice it keeps, called on the type itself.

use std::borrow::BorrowMut;

struct Stack {
    items: Vec<i32>,
}

impl AsMut<[i32]> for Stack {
    fn as_mut(&mut self) -> &mut [i32] {
        &mut self.items
    }
}

impl BorrowMut<[i32]> for Stack {
    fn borrow_mut(&mut self) -> &mut [i32] {
        self.items.as_mut_slice()
    }
}

// `BorrowMut` needs `Borrow`, its supertrait.
impl std::borrow::Borrow<[i32]> for Stack {
    fn borrow(&self) -> &[i32] {
        &self.items
    }
}

struct Grid([u8; 4]);

impl AsMut<[u8]> for Grid {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

fn main() {
    let mut stack = Stack { items: vec![3, 1, 2] };
    stack.as_mut()[0] = 9;
    println!("{:?}", stack.items);
    let slice: &mut [i32] = stack.borrow_mut();
    slice.sort();
    slice[2] += 1;
    println!("{:?}", stack.items);
    let all: &mut [i32] = stack.as_mut();
    all.reverse();
    println!("{:?} {}", stack.items, std::borrow::Borrow::<[i32]>::borrow(&stack).len());

    let mut grid = Grid([0; 4]);
    for (i, cell) in grid.as_mut().iter_mut().enumerate() {
        *cell = i as u8 * 2;
    }
    grid.as_mut().swap(0, 3);
    println!("{:?}", grid.0);
}
