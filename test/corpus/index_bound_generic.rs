//@ library-differs
// A bound of `Index<J>` or `IndexMut<J>` on a `Vec<T>`, a slice or an
// array, by an index generic code is given: its `J: SliceIndex<[T]>`'s
// dictionary, an item or a range of them, as std's one impl of them holds
// by (ADR 0534).
use std::ops::{Index, IndexMut};

struct Buffer<T> {
    buffer: Vec<T>,
}

impl<T, J> Index<J> for Buffer<T>
where
    Vec<T>: Index<J>,
{
    type Output = <Vec<T> as Index<J>>::Output;

    fn index(&self, index: J) -> &Self::Output {
        self.buffer.index(index)
    }
}

impl<T, J> IndexMut<J> for Buffer<T>
where
    Vec<T>: IndexMut<J>,
{
    fn index_mut(&mut self, index: J) -> &mut Self::Output {
        self.buffer.index_mut(index)
    }
}

fn pick<J>(items: &[u32], at: J) -> &<[u32] as Index<J>>::Output
where
    [u32]: Index<J>,
{
    &items[at]
}

fn shout<J>(items: &mut [String; 3], at: J)
where
    [String; 3]: IndexMut<J, Output = String>,
{
    items[at].push('!');
}

fn reversed_len<J: Clone>(items: &mut Vec<u32>, part: J) -> usize
where
    Vec<u32>: Index<J, Output = [u32]> + IndexMut<J>,
{
    items[part.clone()].reverse();
    items[part].len()
}

fn second<T: Copy>(items: &Vec<T>) -> T
where
    Vec<T>: Index<usize, Output = T>,
{
    items[1]
}

fn main() {
    let mut b = Buffer { buffer: vec![1, 2, 3, 4] };
    println!("{} {:?} {:?} {:?}", b[1], &b[1..3], &b[..], &b[2..=3]);
    b[0] = 10;
    b[1..3].reverse();
    println!("{:?}", &b[..2]);
    println!("{} {:?}", pick(&[5, 6, 7], 2), pick(&[5, 6, 7], ..2));
    let mut words = ["a".to_string(), "b".to_string(), "c".to_string()];
    shout(&mut words, 1);
    println!("{words:?}");
    let mut v = vec![1, 2, 3, 4, 5];
    println!("{} {v:?}", reversed_len(&mut v, 1..4));
    println!("{} {v:?}", reversed_len(&mut v, ..));
    println!("{}", second(&v));
}
