// A library's trait whose manifest says its dictionaries carry no constant
// size of its items: generic code's flatten of them, collected where the
// crate's own `size_hint` prints, is made in place, as it was, not held
// and refused for a size no caller gives (#1327, #1341), its `Range`'s
// none, so std asks the hint as it does here.
use dep::Source;

struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint {}", self.0);
        (self.0 as usize, Some(self.0 as usize))
    }
}

fn gathered<S: Source>() -> Vec<u32> {
    Own(4).flat_map(|n| S::items(n)).collect()
}

fn mapped<S: Source>() -> Vec<u32> {
    Own(3).flat_map(|n| S::items(n)).map(|x| x * 10).collect()
}

pub fn main() {
    println!("{:?}", gathered::<dep::Ranges>());
    println!("{:?}", mapped::<dep::Ranges>());
}
