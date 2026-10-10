// A map's or a set's own methods (ADR 0325): a `Map`, a `Set`, or a
// `$KeyMap` or `$KeySet` of keys found by value, the same to each.

// `m.retain(f)`: the entries `f` keeps, visited in `entries`' order, a
// B-tree's sorted, `f` given a handle on a number's or a string's value.
function $retainMap(m, f, entries, handles) {
  for (const [key, value] of handles ? $mutEntries(m, entries) : Array.from(entries)) {
    if (!f(key, value)) m.delete(key);
  }
}

function $retainSet(s, f, items) {
  for (const item of Array.from(items)) if (!f(item)) s.delete(item);
}

// `m.drain()`: its entries, or its items, and it empty.
function $drainAll(m) {
  const items = Array.from(m);
  m.clear();
  return items;
}

function $getKeyValue(m, key) {
  return m.has(key) ? [key, m.get(key)] : undefined;
}

function $removeEntry(m, key) {
  if (!m.has(key)) return undefined;
  const value = m.get(key);
  m.delete(key);
  return [key, value];
}

// `s.replace(x)`: the item equal to `x` that was there, or `None`, and `x`
// in its place.
function $setReplace(s, x) {
  const had = s.has(x);
  s.delete(x);
  s.add(x);
  return had ? x : undefined;
}

// A B-tree's first or last entry, `[key, value]`, or a set's item, by its
// keys' order `cmp`; `undefined` where it's empty. `pop`: taken out too.
function $treeEnd(m, cmp, last, set, pop) {
  let found;
  let seen = false;
  for (const entry of m) {
    const key = set ? entry : entry[0];
    const order = seen ? cmp(key, set ? found : found[0]) : 0;
    if (!seen || (last ? order > 0 : order < 0)) {
      found = entry;
      seen = true;
    }
  }
  if (pop && seen) m.delete(set ? found : found[0]);
  return found;
}

// `m.range(..)` of a B-tree: its entries, or items, whose keys are in the
// bounds, in order; a start past the end panics, as std's does.
// Checked as `range` checks its bounds where `checked`; `extract_if`'s
// aren't (ADR 0344).
function $treeRange(m, cmp, set, name, hasStart, start, hasEnd, end, endIncluded, checked = true) {
  if (checked && hasStart && hasEnd && cmp(start, end) > 0) {
    throw new Error(`range start is greater than range end in ${name}`);
  }
  const sorted = set ? $sortedKeys(m, cmp) : $sortedEntries(m, cmp);
  return sorted.filter((entry) => {
    const key = set ? entry : entry[0];
    if (hasStart && cmp(key, start) < 0) return false;
    if (hasEnd && (endIncluded ? cmp(key, end) > 0 : cmp(key, end) >= 0)) return false;
    return true;
  });
}

// `m.split_off(&key)` of a B-tree: those from `key` on, taken out, in a map
// of its own.
function $treeSplitOff(m, key, cmp, set) {
  const rest = new m.constructor();
  for (const entry of Array.from(m)) {
    const k = set ? entry : entry[0];
    if (cmp(k, key) >= 0) {
      if (set) rest.add(entry);
      else rest.set(k, entry[1]);
      m.delete(k);
    }
  }
  return rest;
}

// `m.append(&mut other)`: all of `other`'s in `m`, its values over `m`'s,
// and `other` empty.
function $treeAppend(m, other, set) {
  for (const entry of other) {
    if (set) m.add(entry);
    else m.set(entry[0], entry[1]);
  }
  other.clear();
}

// A set's `union`, `intersection`, `difference` or `symmetric_difference`:
// its items, a B-tree's sorted by `cmp`.
function $setAlgebra(a, b, op, cmp) {
  const only = (x, y) => Array.from(x).filter((item) => !y.has(item));
  const items =
    op === "union"
      ? [...a, ...only(b, a)]
      : op === "intersection"
        ? Array.from(a).filter((item) => b.has(item))
        : op === "difference"
          ? only(a, b)
          : [...only(a, b), ...only(b, a)];
  return cmp ? items.sort(cmp) : items;
}

// `a.is_subset(&b)`, or `a.is_superset(&b)`: each of the one's in the other.
function $isSubset(a, b, superset) {
  const [inner, outer] = superset ? [b, a] : [a, b];
  for (const item of inner) if (!outer.has(item)) return false;
  return true;
}

function $isDisjoint(a, b) {
  for (const item of a) if (b.has(item)) return false;
  return true;
}

// `m.extend(items)`: each pair set, a later one's value over an earlier's,
// or each item added.
function $extendMap(m, items, set) {
  for (const item of items) {
    if (set) m.add(item);
    else m.set(item[0], item[1]);
  }
}

// A B-tree's `first_entry()`, or `last_entry()` (`last`): its least or
// greatest key's `OccupiedEntry`, `[m, key]`, or `undefined` (ADR 0345).
function $endEntry(m, cmp, last) {
  let end;
  for (const key of m.keys()) {
    if (end === undefined || (last ? cmp(key, end[1]) > 0 : cmp(key, end[1]) < 0)) end = [m, key];
  }
  return end;
}

// An `OccupiedEntry`'s `get()`, or a handle on what's there (`handle`).
function $entryGet([m, key], handle) {
  return handle ? $mutGet(m, key) : m.get(key);
}

// Its `insert(v)`: what was there.
function $entryInsert([m, key], value) {
  const old = m.get(key);
  m.set(key, value);
  return old;
}

// Its `remove()`, or `remove_entry()` (`entry`): what's taken out.
function $entryRemove([m, key], entry) {
  const value = m.get(key);
  m.delete(key);
  return entry ? [key, value] : value;
}

// `m.entry(key)` kept or matched: `Occupied` or `Vacant`, as its key is
// there or not, of `[m, key]`, which its methods are of (ADR 0345).
function $entry(m, key) {
  return { TAG: m.has(key) ? "Occupied" : "Vacant", _0: [m, key] };
}

// A `VacantEntry`'s `insert(v)`: `v` put in, and a `&mut` to it, a handle
// on a number or text (`handle`).
function $vacantInsert([m, key], value, handle) {
  m.set(key, value);
  return handle ? $mutGet(m, key) : value;
}
