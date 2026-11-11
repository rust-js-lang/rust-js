//@ library-differs
// `&self.slice[i]` of an `S: Index<usize> + ?Sized`, rand 0.7's
// `SliceChooseIter`: its dictionary's `index`, the crate's own impl's or
// std's of a slice, an array, a `Vec` and a `VecDeque`, each checked as
// `v[i]` is, its panic std's (#1420).

use std::collections::VecDeque;
use std::ops::Index;
use std::panic;

struct Picked<'a, S: Index<usize> + ?Sized + 'a> {
    slice: &'a S,
    indices: std::vec::IntoIter<usize>,
}

impl<'a, S: Index<usize> + ?Sized + 'a> Iterator for Picked<'a, S> {
    type Item = &'a S::Output;

    fn next(&mut self) -> Option<Self::Item> {
        self.indices.next().map(|i| &self.slice[i])
    }
}

fn pick<S: Index<usize> + ?Sized>(slice: &S, indices: Vec<usize>) -> Picked<'_, S> {
    Picked {
        slice,
        indices: indices.into_iter(),
    }
}

fn first<S>(items: &S) -> &S::Output
where
    S: Index<usize> + ?Sized,
{
    &items[0]
}

struct Squares;

impl Index<usize> for Squares {
    type Output = usize;

    fn index(&self, i: usize) -> &usize {
        const TABLE: [usize; 5] = [0, 1, 4, 9, 16];
        &TABLE[i]
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let words = ["a", "b", "c", "d"];
    let got: Vec<&&str> = pick(&words[..], vec![3, 0, 2]).collect();
    println!("{got:?}");
    let got: Vec<&&str> = pick(&words, vec![1, 1]).collect();
    println!("{got:?}");
    let v = vec![10, 20, 30];
    let got: Vec<&i32> = pick(&v, vec![2, 2, 1]).collect();
    println!("{got:?}");
    let names = vec![String::from("x"), String::from("yz")];
    let owned: Vec<String> = pick(&names, vec![1, 0]).cloned().collect();
    println!("{owned:?} {}", first(&names[..]));
    let points = vec![Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    let p = first(&points).clone();
    println!("{p:?} {}", pick(&points, vec![1]).next() == Some(&Point { x: 3, y: 4 }));
    let mut deque: VecDeque<u8> = VecDeque::new();
    deque.push_back(7);
    deque.push_front(6);
    let got: Vec<&u8> = pick(&deque, vec![0, 1]).collect();
    println!("{got:?}");
    let got: Vec<&usize> = pick(&Squares, vec![4, 1]).collect();
    println!("{got:?} {}", first(&Squares));
    let message = |caught: Box<dyn std::any::Any + Send>| caught.downcast_ref::<String>().cloned();
    let out_of_bounds = panic::catch_unwind(|| pick(&v, vec![0, 5]).count());
    println!("{:?}", out_of_bounds.map_err(message));
    let out_of_bounds = panic::catch_unwind(|| pick(&deque, vec![2]).count());
    println!("{:?}", out_of_bounds.map_err(message));
    let empty: [u8; 0] = [];
    let out_of_bounds = panic::catch_unwind(|| *first(&empty[..]));
    println!("{:?}", out_of_bounds.map_err(message));
}
