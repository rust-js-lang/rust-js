// A value made of parts in turn, a struct's or a tuple's derived clone or
// default: what `build(made)` makes, where each part given to `made(part,
// drop)` as it's made is dropped by its `drop` as a later part's making
// throws, a panic, the latest first, as Rust drops an aggregate's operands
// made so far as a later one unwinds; then what was thrown goes on (#897,
// #902).
function $madeInTurn(build) {
  const drops = [];
  const made = (part, drop) => {
    drops.unshift(() => drop?.(part));
    return part;
  };
  return $unwinds(
    () => build(made),
    () => drops.forEach((drop) => drop()),
  );
}

// `Array.from(items, make)`, a clone of a slice's items, a `VecDeque`'s or
// a map's entries: where a `make` throws, a panic, what it made already is
// dropped by `drop`, in order, as std's clone drops what its guard holds;
// then what was thrown goes on (#897). Of a B-tree of std's nodes, as its
// `clone_subtree` makes them, node by node (`madeEach`), a map's entry
// made before a later edge's panic dropped as its key and value are,
// `dropKey` and `dropValue` (#926).
function $madeEach(items, make, drop, dropKey, dropValue) {
  const tree = items instanceof Set ? items.items : items;
  if (tree?.madeEach !== undefined) {
    if (items instanceof Set) return tree.madeEach(([item]) => make(item), drop, drop);
    const held = (entry) =>
      $inTurn(
        () => dropValue?.(entry[1]),
        () => dropKey?.(entry[0]),
      );
    return tree.madeEach(make, drop, held);
  }
  const made = [];
  $unwinds(
    () => {
      for (const item of items) made.push(make(item));
    },
    () => $dropItems(made, drop),
  );
  return made;
}
