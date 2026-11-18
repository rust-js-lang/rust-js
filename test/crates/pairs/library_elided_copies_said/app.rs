//! A consumer that changes in place what a library's copies give it.
use dep::{Grid, Holder, Spot};

pub fn main() {
    let spots = [Spot { x: 1 }, Spot { x: 2 }];
    let mut owned = dep::owned_spots(&spots);
    owned[0].x += 5;
    println!("{owned:?} {spots:?}");

    let grid = Grid { cells: [1, 2, 3] };
    let mut copy = dep::grid_of(&grid);
    copy.cells[0] = 9;
    println!("{copy:?} {grid:?}");

    let holder = Holder::new(4);
    let mut spot = holder.spot();
    spot.x -= 1;
    println!("{spot:?} {:?}", holder.spot());
}
