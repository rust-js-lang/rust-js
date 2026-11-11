// A map's or a set's own methods (ADR 0325): a `Map`, a `Set`, or a
// `$KeyMap` or `$KeySet` of keys found by value, the same to each.

// The key a map or a set holds equal to `key`, or `undefined`: a
// `$KeyMap`'s, or the key itself, a `Map`'s (ADR 0467). Of a `Map`, only
// ever asked of a key it holds: the lowering asks it of a set that finds
// others by its items alone, each a `$KeySet` or a tree, and the helpers
// here of a key they've found it holds.
function $storedKey(m, key) {
  if (m.storedKey !== undefined) return m.storedKey(key);
  return key;
}

// One search of `m` for `key`: whether it's there, the key it holds equal
// to it and its value. A B-tree of its keys' own order searches once, as
// std's does, its `cmp` asked what std's is (#911); another map, by `has`,
// which asks the program nothing.
function $found(m, key) {
  if (m.found !== undefined) return m.found(key);
  const there = m.has(key);
  return { there, key: there ? $storedKey(m, key) : undefined, value: there ? m.get?.(key) : undefined };
}

// What `$found` found given `value`, or `key` put where it would go, a
// set's item, `value` none; or taken out: no search again.
function $putFound(m, found, key, value) {
  if (m.putFound !== undefined) m.putFound(found, key, value);
  else if (m instanceof Set) m.add(key);
  else m.set(key, value);
}

function $deleteFound(m, found, key) {
  if (m.deleteFound !== undefined) m.deleteFound(found);
  else m.delete(key);
}

// The key `m` holds, met as it's walked, taken out: of a B-tree of its
// keys' own order, by what it is, asking nothing, as std's `retain` takes
// out what it walks to (#911).
function $deleteHeld(m, key) {
  if (m.deleteHeld !== undefined) m.deleteHeld(key);
  else m.delete(key);
}

// `m.retain(f)`: the entries `f` keeps, visited in `entries`' order, a
// B-tree's sorted, `f` given a handle on a number's or a string's value; of
// keys and values with a destructor, each it doesn't keep dropped by
// `dropKey` and `drop` before the next is asked (ADRs 0321, 0467).
function $retainMap(m, f, entries, handles, drop, dropKey) {
  for (const [key, value] of handles ? $mutEntries(m, entries) : Array.from(entries)) {
    if (!f(key, value)) {
      // The value the map holds, not the handle `f` was given on it, which
      // reads the map, where it's gone once deleted (#905).
      const held = handles ? value.value : value;
      $deleteHeld(m, key);
      // Its value dropped though its key's drop panics, as std's of the
      // pair goes on (#416).
      try {
        dropKey?.(key);
      } finally {
        drop?.(held);
      }
    }
  }
}

function $retainSet(s, f, items, drop) {
  for (const item of Array.from(items)) {
    if (!f(item)) {
      $deleteHeld(s, item);
      drop?.(item);
    }
  }
}

// A set's `remove(x)` of items with a destructor: the one it held dropped
// by `drop` (ADR 0467).
function $removeItem(s, item, drop) {
  const found = $found(s, item);
  if (!found.there) return false;
  $deleteFound(s, found, item);
  drop(found.key);
  return true;
}

// `m.drain()`: its entries, or its items, and it empty.
function $drainAll(m) {
  const items = Array.from(m);
  m.clear();
  return items;
}

function $getKeyValue(m, key) {
  const found = $found(m, key);
  return found.there ? [found.key, found.value] : undefined;
}

function $removeEntry(m, key) {
  const found = $found(m, key);
  if (!found.there) return undefined;
  $deleteFound(m, found, key);
  return [found.key, found.value];
}

// `s.replace(x)`: the item equal to `x` that was there, or `None`, and `x`
// in its place, one search; of items with a destructor whose order may
// panic, `x` dropped by `drop` as a comparison does, as std's `replace`
// drops what it was given (#911).
function $setReplace(s, x, drop) {
  const found = $unwinds(
    () => $found(s, x),
    drop && (() => drop(x)),
  );
  // A B-tree's in its place, `mem::replace` of the key it holds.
  if (found.there && s.replaceFound !== undefined) {
    s.replaceFound(found, x);
    return found.key;
  }
  $deleteFound(s, found, x);
  $putFound(s, { ...found, there: false }, x);
  return found.key;
}

// `s.take(x)` of items it finds others by: the one it held, taken out.
function $setTake(s, x) {
  const found = $found(s, x);
  $deleteFound(s, found, x);
  return found.key;
}

// A B-tree's first or last entry, `[key, value]`, or a set's item, by its
// keys' order `cmp`; `undefined` where it's empty. `pop`: taken out too.
// `boxed`: a set's item that may look like `None`, boxed (ADR 0051).
function $treeEnd(m, cmp, last, set, pop, boxed = false) {
  // Of its keys' own order, its first or last as it holds them, asking
  // nothing, as std's walk down its edge asks nothing (#926).
  if (m.endFound !== undefined) {
    const end = m.endFound(last);
    if (end === undefined) return undefined;
    if (pop) m.deleteFound(end);
    const item = set ? end.key : [end.key, end.value];
    return boxed ? $some(item) : item;
  }
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
  return seen && boxed ? $some(found) : found;
}

// `m.range(..)` of a B-tree: its entries, or items, whose keys are in the
// bounds, `start` and `end`, `Bound`s, `{ TAG: "Included", _0: key }` or
// `"Unbounded"`, in order; a start past the end panics, as std's does, and
// so does one equal to it that both exclude. Checked as `range` checks its
// bounds where `checked`; `extract_if`'s aren't (ADR 0344).
// Of a tree of its keys' own order whose order can be seen, std's search,
// whose bounds are checked by `eq` and `ord`'s `>`, as std's are (#926).
function $treeRange(m, cmp, set, name, start, end, checked = true, eq, ord) {
  const tree = set ? m.items : m;
  if (tree?.range !== undefined) {
    // Unchecked, `extract_if`'s: the range it walks, unsearched.
    if (!checked) return { start, end, ord };
    const found = tree.range(start, end, name, eq, ord, checked);
    return found.map(([key, value]) => (set ? key : [key, value]));
  }
  if (checked && start !== "Unbounded" && end !== "Unbounded") {
    const order = cmp(start._0, end._0);
    if (order > 0) {
      throw new Error(`range start is greater than range end in ${name}`);
    }
    if (order === 0 && start.TAG === "Excluded" && end.TAG === "Excluded") {
      throw new Error(`range start and end are equal and excluded in ${name}`);
    }
  }
  const sorted = set ? $sortedKeys(m, cmp) : $sortedEntries(m, cmp);
  return sorted.filter((entry) => {
    const key = set ? entry : entry[0];
    if (start !== "Unbounded") {
      const order = cmp(key, start._0);
      if (start.TAG === "Included" ? order < 0 : order <= 0) return false;
    }
    if (end !== "Unbounded") {
      const order = cmp(key, end._0);
      if (end.TAG === "Included" ? order > 0 : order >= 0) return false;
    }
    return true;
  });
}

// `m.split_off(&key)` of a B-tree: those from `key` on, taken out, in a map
// of its own.
function $treeSplitOff(m, key, cmp, set) {
  // Of its keys' own order, its own search, as std's split's (#911, #926).
  if (m.splitOff !== undefined) return m.splitOff(key);
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
// and `other` empty, what it replaces dropped as std's merge drops it,
// `$merging`'s (#926). Of a tree whose keys' order can be seen, std's
// merge itself.
function $treeAppend(m, other, set, drop, dropKey) {
  if (set ? m.items?.append !== undefined : m.append !== undefined) {
    if (set) m.items.append(other.items, undefined, dropKey);
    else m.append(other, drop, dropKey);
    return;
  }
  const items = Array.from(other, (entry) => (set ? [entry, entry] : entry));
  other.clear();
  $merging(items, drop, dropKey, ([key, value]) => {
    if (!m.has(key)) {
      if (set) m.add(key);
      else m.set(key, value);
      return undefined;
    }
    const own = $storedKey(m, key);
    const held = [own, set ? own : m.get(key)];
    m.delete(key);
    return { held, putBack: () => (set ? m.add(own) : m.set(own, value)) };
  });
}

// std's `merge`'s drops (`append`'s, #926), of `other`'s entries `items`,
// `[key, value]`, in order, each given to `step(entry, first, unwinding)`,
// which puts it in, or, where an equal key's held, takes that entry out
// and gives it, `{ held, putBack }`: its value dropped by `drop`, the
// conflict's, then `putBack()` puts its key back with the entry's value,
// then the entry's key is dropped by `dropKey` as its loop's turn ends, the
// first's at `merge`'s end, its binding's. As `step` panics, a comparison,
// `unwinding` drops what std's frames hold: the entry's value, then its
// key, then the rest of `other`'s, each its key then its value, then the
// first's key where it's held. As a held value's drop panics, its key,
// then the entry's, whose value std's return slot leaks, then the same.
function $merging(items, drop, dropKey, step) {
  let firstHeld = false;
  const firstKey = () => {
    if (firstHeld) dropKey?.(items[0][0]);
  };
  const rest = (n) => () => $dropItems(items.slice(n), $entryDropping(false, drop, dropKey));
  for (let n = 0; n < items.length; n++) {
    const entry = items[n];
    const first = n === 0;
    const after = first ? [rest(1)] : [rest(n + 1), firstKey];
    const unwinding = () =>
      $inTurn(
        () => drop?.(entry[1]),
        () => dropKey?.(entry[0]),
        ...after,
      );
    const taken = step(entry, first, unwinding);
    if (taken === undefined) continue;
    $unwinds(
      () => drop?.(taken.held[1]),
      () => dropKey?.(taken.held[0]),
      () => dropKey?.(entry[0]),
      ...after,
    );
    taken.putBack();
    if (first) firstHeld = true;
    else $unwinds(() => dropKey?.(entry[0]), ...after);
  }
  firstKey();
}

// A set's `union`, `intersection`, `difference` or `symmetric_difference`:
// its items, a B-tree's sorted by `cmp`.
function $setAlgebra(a, b, op, cmp) {
  const only = (x, y) => Array.from(x).filter((item) => !y.has(item));
  const items =
    op === "union"
      ? Array.from(a).concat(only(b, a))
      : op === "intersection"
        ? Array.from(a).filter((item) => b.has(item))
        : op === "difference"
          ? only(a, b)
          : only(a, b).concat(only(b, a));
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
// or each item added; of values with a destructor, the one replaced dropped
// by `drop`, as `insert` gives it back (ADR 0321), and of keys or items with
// one, the one given where an equal's there by `dropKey` (ADR 0467).
//
// Where stepping `items` throws, a panic in a closure, what's left of them
// closed, as Rust drops the iterator `extend` took as it unwinds (#423): by
// its own `fold`, which closes it, or by `$eachOf`, as `for..of` doesn't
// where its own step throws (#1141). Of an
// iterator with a `fold` of its own, an adapter's, the crate's or
// `$folded`'s, by it, as std's `extend` is its `for_each`: so what it
// replaces is dropped as each comes, before what its source drops at its
// end, and a `skip`'s first step is its `nth(n - 1)` (#494). Of an array,
// a `Vec` or an array it took, what's left of it dropped as a put unwinds,
// a drop of what it replaces, each entry by `$entryDropping`, as std's
// `for_each` drops the `IntoIter` it was given (#900). A B-tree's
// (`sorted`) drops the key given before it writes the value, as `insert`
// does, `$insert`'s (#921).
function $extendMap(m, items, set, drop, dropKey, sorted) {
  const put = (item) => {
    // Of its keys' own order, `insert`'s one search (#926).
    if (m.found !== undefined) {
      if (set) {
        $add(m, item, dropKey);
        return;
      }
      const [key, value] = item;
      const found = $unwinds(
        () => m.found(key),
        dropKey && (() => dropKey(key)),
        drop && (() => drop(value)),
      );
      if (found.there) $unwinds(() => dropKey?.(key), drop && (() => drop(value)));
      m.putFound(found, key, value);
      if (found.there) drop?.(found.value);
      return;
    }
    if (set) {
      if (m.has(item)) dropKey?.(item);
      else m.add(item);
      return;
    }
    const replaced = m.has(item[0]);
    if (sorted && replaced) $unwinds(() => dropKey(item[0]), drop && (() => drop(item[1])));
    const old = m.get(item[0]);
    m.set(item[0], item[1]);
    if (replaced) {
      if (!sorted) dropKey?.(item[0]);
      drop?.(old);
    }
  };
  if (Object.hasOwn(items, "fold")) {
    items.fold(undefined, (_, item) => put(item));
    return m;
  }
  if (Array.isArray(items)) {
    let i = 0;
    $unwinds(
      () => {
        for (; i < items.length; i++) put(items[i]);
      },
      () => $dropItems(items.slice(i + 1), $entryDropping(set, drop, dropKey)),
    );
    return m;
  }
  $eachOf(items, put);
  return m;
}

// What drops an entry of a map, `[key, value]`, its key, then its value
// though the key's drop panics, as Rust drops a pair; of a set, its item.
function $entryDropping(set, drop, dropKey) {
  if (set) return (item) => dropKey?.(item);
  return ([key, value]) =>
    $inTurn(
      () => dropKey?.(key),
      () => drop?.(value),
    );
}

// `collect()` into a new hash map or set, `m`, of `items` a panic may stop
// as they're stepped: `$extendMap`'s, and where it throws, what's left
// closed, then each entry `m` holds dropped, its key then its value, as
// std's `from_iter` drops its `extend`'s iterator, then its map (#423).
function $mapFrom(m, items, set, drop, dropKey) {
  return $unwinds(
    () => $extendMap(m, items, set, drop, dropKey),
    () => $dropItems(set ? m.keys() : m.entries(), $entryDropping(set, drop, dropKey)),
  );
}

// A B-tree's `first_entry()`, or `last_entry()` (`last`): its least or
// greatest key's `OccupiedEntry`, `[m, key]`, or `undefined` (ADR 0345).
function $endEntry(m, cmp, last) {
  if (m.endFound !== undefined) {
    const end = m.endFound(last);
    return end && [m, end.key, end];
  }
  let end;
  for (const key of m.keys()) {
    if (end === undefined || (last ? cmp(key, end[1]) > 0 : cmp(key, end[1]) < 0)) end = [m, key];
  }
  return end;
}

// An `OccupiedEntry`'s `get()`, or a handle on what's there (`handle`):
// of a map of its keys' own order, of what its search `found`, asking
// nothing again, as std's entry holds where it is (#926).
function $entryGet([m, key, found], handle) {
  if (found !== undefined) return handle ? $entryHandle(found.entry) : found.entry[1];
  return handle ? $mutGet(m, key) : m.get(key);
}

// Its `insert(v)`: what was there.
function $entryInsert([m, key, found], value) {
  if (found !== undefined) {
    const old = found.entry[1];
    found.entry[1] = value;
    return old;
  }
  const old = m.get(key);
  m.set(key, value);
  return old;
}

// Its `remove()`, or `remove_entry()` (`entry`): what's taken out.
function $entryRemove([m, key, found], entry) {
  if (found !== undefined) {
    m.deleteFound(found);
    return entry ? [found.key, found.entry[1]] : found.entry[1];
  }
  const value = m.get(key);
  m.delete(key);
  return entry ? [key, value] : value;
}

// `m.entry(key)` kept or matched: `Occupied` or `Vacant`, as its key is
// there or not, of `[m, key]`, the key it holds where it's there, which its
// methods are of (ADR 0345); where
// it's there, the key given dropped by `dropKey`, of keys with a destructor
// (ADR 0467).
// Of a map of its keys' own order, one search, its key given dropped as
// the search panics, and what it found kept for the entry's methods: `[m,
// key, found]` (#926).
function $entry(m, key, dropKey) {
  if (m.found !== undefined) {
    const found = $unwinds(
      () => m.found(key),
      dropKey && (() => dropKey(key)),
    );
    if (found.there) dropKey?.(key);
    return { TAG: found.there ? "Occupied" : "Vacant", _0: [m, found.there ? found.key : key, found] };
  }
  const there = m.has(key);
  if (there) dropKey?.(key);
  return { TAG: there ? "Occupied" : "Vacant", _0: [m, there ? $storedKey(m, key) : key] };
}

// An entry's `and_modify(f)`: `f` given its value where its key is there,
// a handle on a number or text (`handle`), and the entry, for `or_insert`.
function $andModify(entry, f, handle) {
  if (entry.TAG === "Occupied") f($entryGet(entry._0, handle));
  return entry;
}

// A `VacantEntry`'s `insert(v)`: `v` put in, and a `&mut` to it, a handle
// on a number or text (`handle`).
function $vacantInsert([m, key, found], value, handle) {
  if (found !== undefined) {
    const entry = m.putFound(found, key, value);
    return handle ? $entryHandle(entry) : value;
  }
  m.set(key, value);
  return handle ? $mutGet(m, key) : value;
}

// A `VacantEntry`'s `insert_entry(v)`: `v` put in, and the
// `OccupiedEntry` of it, of what its search found, marked there, so no
// search asks its key's `hash` again (#1369).
function $vacantInsertEntry([m, key, found], value) {
  if (found !== undefined) {
    const entry = m.putFound(found, key, value);
    return [m, key, { ...found, there: true, key, value, entry }];
  }
  m.set(key, value);
  return [m, key];
}

// An entry's `insert_entry(v)`: where its key is there, `v` written and
// what was there dropped by `drop`, as std's `OccupiedEntry::insert` gives
// it back to drop; else put in. Its `OccupiedEntry` (#1369).
function $entryInsertEntry(entry, value, drop) {
  if (entry.TAG === "Occupied") {
    // Written first: `drop?.(..)` of no `drop` evaluates no argument.
    const old = $entryInsert(entry._0, value);
    drop?.(old);
    return entry._0;
  }
  return $vacantInsertEntry(entry._0, value);
}

// An entry's `or_insert(value)`, `$entry`'s, of a map whose search can be
// seen (#926): what `$entry`'s search found written, no search again. Its
// value, or a handle on it (`handle`); where it's there, `value` dropped by
// `drop`.
function $entryOrInsert(entry, value, handle, drop) {
  if (entry.TAG === "Occupied") {
    drop?.(value);
    return $entryGet(entry._0, handle);
  }
  return $vacantInsert(entry._0, value, handle);
}

// Its `or_insert_with(make)`, or `or_insert_with_key` (`keyed`), `make`
// given its key: where it's there, `make` dropped unrun by `dropMake`;
// where it isn't and `make` panics, the key by `dropKey`, as std's
// `VacantEntry` drops it.
function $entryOrInsertWith(entry, make, handle, dropKey, dropMake, keyed) {
  if (entry.TAG === "Occupied") {
    dropMake?.(make);
    return $entryGet(entry._0, handle);
  }
  const key = entry._0[1];
  const value = $unwinds(
    () => (keyed ? make(key) : make()),
    dropKey && (() => dropKey(key)),
  );
  return $vacantInsert(entry._0, value, handle);
}

// A B-tree made of `items`, its entries or a set's items, sorted by their
// keys' `cmp`, and of equal ones the last kept, as std's sorts them and
// keeps the last, where `insert` keeps the first (ADR 0401); the earlier
// dropped, its key by `dropKey`, then its value by `drop` (ADR 0506).
function $treeFrom(tree, items, set, cmp, drop, dropKey) {
  const key = (item) => (set ? item : item[0]);
  for (const item of Array.from(items).sort((a, b) => cmp(key(a), key(b)))) {
    if (tree.has(key(item))) {
      const old = set ? undefined : tree.get(item[0]);
      dropKey?.($storedKey(tree, key(item)));
      tree.delete(key(item));
      if (!set) drop?.(old);
    }
    if (set) tree.add(item);
    else tree.set(item[0], item[1]);
  }
  return tree;
}

// A B-tree made of `items` whose keys' order may be seen, of keys whose
// order is seen or not total, or a type parameter's (#926): of std's nodes,
// std's bulk build. Its entries sorted by std's stable sort, `kind` and
// `size` its entries', `(K, V)`'s or a set's `T`'s: a map's by `cmp` of
// their keys, a set's by `<`, its `PartialOrd`'s `lt`; as that panics,
// each dropped in the order it's left. Then each kept that `!=` doesn't
// find equal to the next, and bulk-pushed. `TOrd` is the keys' `Ord`
// dictionary, whose `PartialOrd()`, and its `PartialEq()`, give `lt` and
// `ne`, an impl's own included (#944, #962). Of another class, a sorted array's or numbers',
// `$treeFrom`'s.
function $treeFromSorted(tree, items, TOrd, kind, size, drop, dropKey) {
  const set = tree instanceof Set;
  const nodes = set ? tree.items : tree;
  if (nodes?.pushSorted === undefined) return $treeFrom(tree, items, set, TOrd.cmp, drop, dropKey);
  const partialOrd = TOrd.PartialOrd();
  const ne = partialOrd.PartialEq().ne;
  const sorted = Array.from(items, (item) => (set ? [item, item] : [item[0], item[1]]));
  if (sorted.length === 0) return tree;
  const dropEntry = set ? dropKey && (([item]) => dropKey(item)) : (drop || dropKey) && $entryDropping(false, drop, dropKey);
  const isLess = set ? (a, b) => partialOrd.lt(a[0], b[0]) : (a, b) => TOrd.cmp(a[0], b[0]) < 0;
  $unwinds(
    () => $sortStable(sorted, isLess, kind, size),
    dropEntry && (() => $dropItems(sorted, dropEntry)),
  );
  nodes.pushSorted(sorted, ne, dropEntry);
  return tree;
}

// A hashed map's or set's entries dropped as hashbrown's table drops them:
// each by `drop`, in turn, a panic stopping it and leaking the rest, the
// table left empty, as its guard leaves it (#920, #922).
function $dropTable(m, drop) {
  try {
    if (drop) for (const entry of m) drop(entry);
  } finally {
    m.clear();
  }
}

// `m.clone_from(&from)` of a hashed map or set, hashbrown's: its entries
// dropped, `$dropTable`; then each of `from`'s cloned by `make`, those made
// dropped by `drop` as a later one panics, `m` left empty, then put in
// (#922).
function $cloneIntoMap(m, from, make, drop) {
  $dropTable(m, drop);
  const made = drop ? $madeEach(from, make ?? ((entry) => entry), drop) : Array.from(from, make);
  for (const entry of made) {
    if (m instanceof Set) m.add(entry);
    else m.set(entry[0], entry[1]);
  }
}

// A hash map's `get_disjoint_mut(keys)`: each key's value, a handle on a
// number or text (`handle`), or `None` where it isn't there; where two
// find one entry, `duplicate keys found`, as std's does once it's found
// them all (#1276). A handle is on what each search found, asking nothing
// again (#1362).
function $mapGetDisjointMut(m, keys, handle) {
  const found = keys.map((key) => $found(m, key));
  found.forEach((each, i) => {
    if (each.there && found.slice(0, i).some((other) => other.there && other.key === each.key)) {
      throw new Error("duplicate keys found");
    }
  });
  const handleOf = (each, i) => (each.entry === undefined ? $mutGet(m, keys[i]) : $entryHandle(each.entry));
  return found.map((each, i) => (!each.there ? undefined : handle ? handleOf(each, i) : each.value));
}
