// `to_owned()` of a `T: Clone`, std's blanket impl, is its `clone()`: a
// `Vec`'s through a reference, a struct's, a generic `T`'s through its
// dictionary; a `str`'s and a slice's are their own. As the activation
// functions copy the vector they're given before changing it.
#[derive(Clone, Debug)]
struct Layer {
    weights: Vec<f64>,
}

fn leaky(vector: &Vec<f64>, alpha: f64) -> Vec<f64> {
    let mut owned = vector.to_owned();
    for value in &mut owned {
        if *value < 0.0 {
            *value *= alpha;
        }
    }
    owned
}

fn both<T: Clone>(x: &T) -> (T, T) {
    (x.to_owned(), x.clone())
}

fn main() {
    let input = vec![1.5, -2.0, 0.0];
    println!("{:?} {:?}", leaky(&input, 0.5), input);

    let layer = Layer { weights: vec![0.25] };
    let (mut a, b) = both(&layer);
    a.weights.push(1.0);
    println!("{layer:?} {a:?} {b:?}");

    let s: &[u8] = &[1, 2];
    let mut from_slice = s.to_owned();
    from_slice.push(3);
    println!("{s:?} {from_slice:?} {}", "text".to_owned());
}
