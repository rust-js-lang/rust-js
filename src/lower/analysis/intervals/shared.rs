//! A `State`'s maps and sets, shared by the states that hold them (#2310).
//! The walk keeps a state at each block, and narrowing another: each a
//! copy of what flows into it with a few facts changed. Held whole, each
//! block's was as large as the function's facts so far, a function of
//! 3,000 calls taking 5.35 GB. A clone of one of these shares all of it,
//! and a change copies only the path to what it changes (`imbl`'s
//! persistent trees), so each block holds what's its own.
//!
//! The one owner of what they mean: an `IndexMap`'s and an `IndexSet`'s, so
//! every answer the walk gives is the same. Each walks in the order its
//! keys were first inserted, a key inserted again keeping its place and a
//! removed one leaving the rest's, `shift_remove`'s; and two are equal
//! where they hold the same entries, in any order.

use imbl::OrdMap;

/// A map in the order its keys were first inserted, as an `IndexMap`,
/// whose clones share what they hold.
#[derive(Clone)]
pub(in crate::lower::analysis::intervals) struct SharedMap<K: Ord + Clone, V: Clone> {
    /// Each entry, by its place: the insertion order.
    entries: OrdMap<u64, (K, V)>,
    /// Each key's place.
    places: OrdMap<K, u64>,
    /// The place a new key takes, past every one held.
    next: u64,
}

impl<K: Ord + Clone, V: Clone> Default for SharedMap<K, V> {
    fn default() -> Self {
        SharedMap {
            entries: OrdMap::new(),
            places: OrdMap::new(),
            next: 0,
        }
    }
}

impl<K: Ord + Clone, V: Clone> SharedMap<K, V> {
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn len(&self) -> usize {
        self.places.len()
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn is_empty(&self) -> bool {
        self.places.is_empty()
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn get(&self, key: &K) -> Option<&V> {
        let place = self.places.get(key)?;
        self.entries.get(place).map(|(_, value)| value)
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn contains_key(&self, key: &K) -> bool {
        self.places.contains_key(key)
    }

    /// `key`'s value, `value`: in its place where it's held, last where
    /// it's new. What it held, if any.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.places.get(&key) {
            Some(&place) => self.entries.insert(place, (key, value)).map(|(_, old)| old),
            None => {
                self.places.insert(key.clone(), self.next);
                self.entries.insert(self.next, (key, value));
                self.next += 1;
                None
            }
        }
    }

    /// `key` removed, the rest kept in their order: what it held.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn shift_remove(&mut self, key: &K) -> Option<V> {
        let place = self.places.remove(key)?;
        self.entries.remove(&place).map(|(_, value)| value)
    }

    /// Each entry `keep` refuses removed, the rest kept in their order.
    /// Unchanged, and still shared, where it refuses none.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn retain(&mut self, mut keep: impl FnMut(&K, &V) -> bool) {
        let gone: Vec<K> = (self.entries.values())
            .filter(|(key, value)| !keep(key, value))
            .map(|(key, _)| key.clone())
            .collect();
        for key in &gone {
            self.shift_remove(key);
        }
    }

    /// `key`'s value, to change in place: `value` inserted last where it
    /// has none, `IndexMap::entry(key).or_insert(value)`.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn or_insert(&mut self, key: K, value: V) -> &mut V {
        if !self.places.contains_key(&key) {
            self.insert(key.clone(), value);
        }
        let place = self.places[&key];
        &mut self
            .entries
            .get_mut(&place)
            .expect("each key's place holds its entry")
            .1
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn iter(&self) -> impl DoubleEndedIterator<Item = (&K, &V)> + '_ {
        self.entries.values().map(|(key, value)| (key, value))
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn keys(&self) -> impl DoubleEndedIterator<Item = &K> + '_ {
        self.entries.values().map(|(key, _)| key)
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn values(&self) -> impl DoubleEndedIterator<Item = &V> + '_ {
        self.entries.values().map(|(_, value)| value)
    }
}

/// An `IndexMap`'s equality: the same entries, in any order. Two that
/// share what they hold are equal unread, as a block's state is with what
/// flows into it unchanged.
impl<K: Ord + Clone, V: Clone + PartialEq> PartialEq for SharedMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        (self.entries.ptr_eq(&other.entries))
            || (self.len() == other.len() && self.iter().all(|(key, value)| other.get(key) == Some(value)))
    }
}

impl<K: Ord + Clone, V: Clone> FromIterator<(K, V)> for SharedMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
        let mut map = SharedMap::default();
        for (key, value) in entries {
            map.insert(key, value);
        }
        map
    }
}

impl<K: Ord + Clone, V: Clone> Extend<(K, V)> for SharedMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) {
        for (key, value) in entries {
            self.insert(key, value);
        }
    }
}

impl<'a, K: Ord + Clone, V: Clone> IntoIterator for &'a SharedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Box<dyn DoubleEndedIterator<Item = (&'a K, &'a V)> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

/// A set in the order its items were first inserted, as an `IndexSet`,
/// whose clones share what they hold.
#[derive(Clone, PartialEq)]
pub(in crate::lower::analysis::intervals) struct SharedSet<T: Ord + Clone>(SharedMap<T, ()>);

impl<T: Ord + Clone> Default for SharedSet<T> {
    fn default() -> Self {
        SharedSet(SharedMap::default())
    }
}

impl<T: Ord + Clone> SharedSet<T> {
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn contains(&self, item: &T) -> bool {
        self.0.contains_key(item)
    }

    /// `item` held, last where it's new: whether it is.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn insert(&mut self, item: T) -> bool {
        !self.0.contains_key(&item) && self.0.insert(item, ()).is_none()
    }

    /// `item` removed, the rest kept in their order: whether it was held.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn shift_remove(&mut self, item: &T) -> bool {
        self.0.shift_remove(item).is_some()
    }

    /// Each item `keep` refuses removed, the rest kept in their order.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        self.0.retain(|item, ()| keep(item));
    }

    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + '_ {
        self.0.keys()
    }

    /// Each item of this, then each of `other` this hasn't, as
    /// `IndexSet::union`.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn union<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = &'a T> + 'a {
        self.iter().chain(other.iter().filter(|item| !self.contains(item)))
    }

    /// Each item of this `other` holds too, in this one's order, as
    /// `IndexSet::intersection`.
    // helper: a collection's own operation, an `IndexMap`'s or `IndexSet`'s
    pub(in crate::lower::analysis::intervals) fn intersection<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = &'a T> + 'a {
        self.iter().filter(|item| other.contains(item))
    }
}

impl<T: Ord + Clone> IntoIterator for SharedSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter().cloned().collect::<Vec<T>>().into_iter()
    }
}

impl<T: Ord + Clone> FromIterator<T> for SharedSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        SharedSet(items.into_iter().map(|item| (item, ())).collect())
    }
}

impl<T: Ord + Clone> Extend<T> for SharedSet<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        for item in items {
            self.insert(item);
        }
    }
}

impl<'a, T: Ord + Clone> IntoIterator for &'a SharedSet<T> {
    type Item = &'a T;
    type IntoIter = Box<dyn DoubleEndedIterator<Item = &'a T> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::{SharedMap, SharedSet};
    use rust_js_program::collections::{IndexMap, IndexSet};

    /// A map's entries in its order.
    type Entries = Vec<(u8, u8)>;

    /// A small deterministic generator, xorshift, so a failure replays.
    struct Draws(u64);

    impl Draws {
        fn next(&mut self, below: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % below
        }
    }

    fn entries(map: &SharedMap<u8, u8>) -> Entries {
        map.iter().map(|(&k, &v)| (k, v)).collect()
    }

    fn index_entries(map: &IndexMap<u8, u8>) -> Entries {
        map.iter().map(|(&k, &v)| (k, v)).collect()
    }

    // Each operation the walk asks of a map, drawn at random, answers as an
    // `IndexMap`'s and leaves the same entries in the same order; a clone
    // taken along the way keeps what it was given, whatever the original
    // does after; and two maps are equal where `IndexMap`s of their entries
    // are, in any order.
    #[test]
    fn a_shared_map_answers_as_an_index_map_does() {
        let mut draws = Draws(0x2310);
        let mut shared: SharedMap<u8, u8> = SharedMap::default();
        let mut index: IndexMap<u8, u8> = IndexMap::default();
        let mut kept: Vec<(SharedMap<u8, u8>, Entries)> = Vec::new();
        for step in 0..20_000 {
            let key = draws.next(24) as u8;
            let value = draws.next(4) as u8;
            match draws.next(6) {
                0 | 1 => assert_eq!(
                    shared.insert(key, value),
                    index.insert(key, value),
                    "insert, step {step}"
                ),
                2 => assert_eq!(
                    shared.shift_remove(&key),
                    index.shift_remove(&key),
                    "remove, step {step}"
                ),
                3 => {
                    let parity = value % 2;
                    shared.retain(|&k, &v| (k + v) % 2 != parity);
                    index.retain(|&k, &mut v| (k + v) % 2 != parity);
                }
                4 => {
                    *shared.or_insert(key, value) += 1;
                    *index.entry(key).or_insert(value) += 1;
                }
                _ => kept.push((shared.clone(), index_entries(&index))),
            }
            assert_eq!(entries(&shared), index_entries(&index), "step {step}");
            assert_eq!((shared.len(), shared.is_empty()), (index.len(), index.is_empty()));
            assert_eq!(shared.get(&key), index.get(&key));
            assert_eq!(shared.contains_key(&key), index.contains_key(&key));
            let reordered: SharedMap<u8, u8> = entries(&shared).into_iter().rev().collect();
            assert!(reordered == shared);
            let other: SharedMap<u8, u8> = entries(&shared).into_iter().skip(1).collect();
            let index_other: IndexMap<u8, u8> = index_entries(&index).into_iter().skip(1).collect();
            assert_eq!(other == shared, index_other == index);
        }
        for (clone, was) in &kept {
            assert_eq!(&entries(clone), was);
        }
    }

    // As of a set's: `insert`, `shift_remove` and `retain`, and `union` and
    // `intersection` in `IndexSet`'s orders.
    #[test]
    fn a_shared_set_answers_as_an_index_set_does() {
        let mut draws = Draws(0x2308);
        let mut shared: [SharedSet<u8>; 2] = Default::default();
        let mut index: [IndexSet<u8>; 2] = Default::default();
        for step in 0..20_000 {
            let item = draws.next(24) as u8;
            let at = draws.next(2) as usize;
            match draws.next(3) {
                0 => assert_eq!(shared[at].insert(item), index[at].insert(item), "insert, step {step}"),
                1 => assert_eq!(
                    shared[at].shift_remove(&item),
                    index[at].shift_remove(&item),
                    "remove, step {step}"
                ),
                _ => {
                    shared[at].retain(|&i| i % 5 != item % 5);
                    index[at].retain(|&i| i % 5 != item % 5);
                }
            }
            for at in 0..2 {
                assert_eq!(
                    shared[at].iter().collect::<Vec<_>>(),
                    index[at].iter().collect::<Vec<_>>()
                );
                assert_eq!(shared[at].contains(&item), index[at].contains(&item));
            }
            let [a, b] = &shared;
            let [ia, ib] = &index;
            assert_eq!(a.union(b).collect::<Vec<_>>(), ia.union(ib).collect::<Vec<_>>());
            assert_eq!(
                a.intersection(b).collect::<Vec<_>>(),
                ia.intersection(ib).collect::<Vec<_>>()
            );
            assert_eq!(a == b, ia == ib);
        }
    }
}
