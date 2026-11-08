// `v.extract_if(start..end, f)` of a `Vec`, or a list's of all of it (ADR
// 0344): a JS iterator that, as each item is asked for, asks `f` of the
// next from `start`, a handle on a number or text (`handle`), and takes out
// and gives each it holds of. What it doesn't reach stays. Checked as
// `slice::range` checks, as it's made.
function $extractIf(v, start, end, f, handle = false) {
  end ??= v.length;
  $checkRange(start, end, v.length);
  let at = start;
  return Iterator.from({
    next() {
      while (at < end) {
        if (f(handle ? $mutAt(v, at) : v[at])) {
          end--;
          return { done: false, value: v.splice(at, 1)[0] };
        }
        at++;
      }
      return { done: true, value: undefined };
    },
  });
}

// A map's or a set's `extract_if(f)`: the same, of its entries as `entries`
// has them, a B-tree's in order and in its range, `f` given each key and a
// handle on a value that's a number or text (`handles`), or each item of a
// set (`set`). Each it holds of is taken out and given, a map's as
// `[key, value]`.
// Of a tree whose keys' order can be seen, std's walk (#926): from where
// its range starts, `$treeRange`'s unsearched range of it, or all of it.
function $mapExtractIf(m, f, entries, handles, set, drop) {
  const tree = set ? m.items : m;
  // Closed once, what it holds dropped, `drop`'s, its predicate then its
  // range's bounds, as std's `ExtractIf` drops its `pred` then its
  // `range`, where its owner drops it, or as its consumer steps it to its
  // end (#945).
  const steps = (next) => {
    const it = new $Steps(next, () => drop && $close(it, drop));
    return it;
  };
  if (tree?.extractIf !== undefined) {
    const { start, end, partialCmp } = entries === m ? { start: "Unbounded", end: "Unbounded" } : entries;
    const take = set
      ? ([item]) => f(item)
      : (entry) => f(entry[0], handles ? $entryHandle(entry) : entry[1]);
    const walk = tree.extractIf(start, end, partialCmp, take);
    return steps(() => {
      const entry = walk.next();
      if (entry === undefined) return { done: true, value: undefined };
      return { done: false, value: set ? entry[0] : [entry[0], entry[1]] };
    });
  }
  const each = Array.from(entries)[Symbol.iterator]();
  return steps(() => {
    for (let step = each.next(); !step.done; step = each.next()) {
      if (set) {
        if (!f(step.value)) continue;
        m.delete(step.value);
        return { done: false, value: step.value };
      }
      const [key, value] = step.value;
      if (!f(key, handles ? $mutGet(m, key) : value)) continue;
      const taken = m.get(key);
      m.delete(key);
      return { done: false, value: [key, taken] };
    }
    return { done: true, value: undefined };
  });
}
