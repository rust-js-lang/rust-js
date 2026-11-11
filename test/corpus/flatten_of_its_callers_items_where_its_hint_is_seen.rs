// A generic flatten of its parameter's items, `I::Item`, one of its
// slots, whose constant size its caller gives: collected where its hint
// is seen, of the crate's own iterator whose `size_hint` prints, it's
// held, std's struct, which asks the hint once as std does, where a
// flatten made in place asked it again as the `Vec` grew. A flatten
// whose items' size no caller gives yet is made in place, as it was
// (#1327, #1341). And a flatten given to generic code that collects it,
// as std's `SpecFromIter` asks its hint as the `Vec` grows (#1239).
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

fn flat<I>(it: I) -> Vec<u32>
where
    I: Iterator,
    I::Item: IntoIterator<Item = u32>,
{
    it.flatten().collect()
}

fn gathered<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.collect()
}

fn main() {
    println!("{:?}", flat(Own(4).map(|n| [n, n + 10])));
    println!("{:?}", flat(Own(3).map(|n| vec![n; 3])));
    println!("{:?}", gathered(Own(4).flat_map(|n| [n, n + 10])));
}
