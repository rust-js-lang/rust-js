// A map of a type parameter's keys that std makes: an entry's
// `or_default()`, which gives a `&mut` to the default it made, and
// `map(BTreeMap::from)`, a constructor given as a value. Each is of the
// class its caller gives for the keys, as Dijkstra's tests build their
// graphs of tuple vertices.
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

type Graph<V> = BTreeMap<V, BTreeMap<V, u32>>;

fn add_edge<V: Ord + Copy>(graph: &mut Graph<V>, v1: V, v2: V, c: u32) {
    graph.entry(v1).or_default().insert(v2, c);
    graph.entry(v2).or_default();
}

fn singles<V: Ord + Copy>(pairs: &[(V, u32)]) -> Vec<BTreeMap<V, u32>> {
    pairs.iter().map(|&pair| [pair]).map(BTreeMap::from).collect()
}

fn neighbours<V: Eq + Hash + Copy>(edges: &[(V, V)]) -> HashMap<V, HashMap<V, usize>> {
    let mut all: HashMap<V, HashMap<V, usize>> = HashMap::new();
    for &(a, b) in edges {
        *all.entry(a).or_default().entry(b).or_default() += 1;
    }
    all
}

fn main() {
    let mut graph = Graph::new();
    add_edge(&mut graph, (0, 'a'), (1, 'b'), 4);
    add_edge(&mut graph, (0, 'a'), (2, 'c'), 1);
    add_edge(&mut graph, (2, 'c'), (1, 'b'), 2);
    add_edge(&mut graph, (0, 'a'), (1, 'b'), 3);
    println!("{graph:?}");
    let maps = singles(&[((1, 'x'), 1), ((0, 'y'), 2)]);
    let mut merged = maps[0].clone();
    merged.extend(maps[1].clone());
    merged.insert((1, 'x'), 5);
    println!("{merged:?} {}", merged.contains_key(&(0, 'y')));

    let all = neighbours(&[((1, 1), (2, 2)), ((1, 1), (2, 2)), ((2, 2), (1, 1))]);
    let mut counts: Vec<_> = all
        .iter()
        .flat_map(|(a, next)| next.iter().map(move |(b, n)| (*a, *b, *n)))
        .collect();
    counts.sort();
    println!("{counts:?}");
}
