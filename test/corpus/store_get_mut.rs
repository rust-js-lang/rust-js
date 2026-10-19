// A map over a generic store, litemap's: a `&mut V` a store's method gives
// in what it gives, `lm_get_mut(i).unwrap().1`, handed on and replaced
// through (ADRs 0099, 0147); and `filter_map` of items with a destructor,
// each given to its function, which drops it (ADR 0098). Public, as a
// library's, whose type parameters a consumer may give a destructor.
use std::marker::PhantomData;
use std::mem;

pub trait StoreMut<K, V> {
    fn lm_get_mut(&mut self, i: usize) -> Option<(&K, &mut V)>;
}

impl<K, V> StoreMut<K, V> for Vec<(K, V)> {
    fn lm_get_mut(&mut self, i: usize) -> Option<(&K, &mut V)> {
        self.get_mut(i).map(|(k, v)| (&*k, v))
    }
}

pub struct LiteMap<K, V, S> {
    pub values: S,
    pub _types: PhantomData<(K, V)>,
}

impl<K, V, S: StoreMut<K, V>> LiteMap<K, V, S> {
    pub fn get_mut(&mut self, i: usize) -> Option<&mut V> {
        Some(self.values.lm_get_mut(i)?.1)
    }

    pub fn first_mut(&mut self) -> &mut V {
        self.values.lm_get_mut(0).unwrap().1
    }

    pub fn insert_at(&mut self, i: usize, value: V) -> V {
        mem::replace(self.values.lm_get_mut(i).unwrap().1, value)
    }

    /// litemap's `OccupiedEntry::insert`: through a `&mut V` a method of
    /// its own gives.
    pub fn replace_first(&mut self, value: V) -> V {
        mem::replace(self.first_mut(), value)
    }
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn odd(items: Vec<Noisy>) -> Vec<u32> {
    items
        .into_iter()
        .filter_map(|n| if n.0 % 2 == 1 { Some(n.0) } else { None })
        .collect()
}

fn main() {
    let mut map = LiteMap {
        values: vec![(1, 10u32), (2, 20)],
        _types: PhantomData,
    };
    if let Some(v) = map.get_mut(1) {
        *v += 5;
    }
    *map.first_mut() += 1;
    let old = map.insert_at(0, 99);
    let missing = map.get_mut(7).is_none();
    println!("{}", map.replace_first(7));
    println!("{:?} {old} {missing}", map.values);
    let mut names = LiteMap {
        values: vec![("a", String::from("x"))],
        _types: PhantomData,
    };
    names.first_mut().push('!');
    let was = names.insert_at(0, String::from("y"));
    println!("{:?} {was}", names.values);
    println!("{:?}", odd(vec![Noisy(1), Noisy(2), Noisy(3)]));
}
