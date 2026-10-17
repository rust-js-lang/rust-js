// `Cell::from_mut` sees a `&mut` as a `&Cell`, and `as_slice_of_cells` and
// `as_array_of_cells` a cell of a slice or an array as cells of its items:
// each write changes the place the `&mut` is to, in place (ADR 0153;
// rustc's rfcs/rfc-1789-as-cell/from-mut.rs).
use std::cell::Cell;

fn bump_all(cells: &[Cell<i32>]) {
    for cell in cells {
        cell.set(cell.get() * 10);
    }
}

fn reset(target: &mut [i32; 3]) {
    let cell = Cell::from_mut(target);
    cell.set([7, 8, 9]);
}

fn main() {
    let mut n = 1;
    let cell = Cell::from_mut(&mut n);
    cell.set(cell.get() + 41);
    println!("{n}");

    let slice: &mut [i32] = &mut [1, 2, 3];
    let cells = Cell::from_mut(slice).as_slice_of_cells();
    bump_all(cells);
    println!("{} {:?}", cells.len(), slice);

    let mut array = [1, 2, 3];
    let cells = Cell::from_mut(&mut array).as_array_of_cells();
    cells[0].set(99);
    cells[1].swap(&cells[2]);
    println!("{array:?}");

    reset(&mut array);
    println!("{array:?}");

    let mut words = [String::from("a"), String::from("b")];
    let cells = Cell::from_mut(&mut words).as_array_of_cells();
    cells[1].set(String::from("z"));
    println!("{words:?}");
}
