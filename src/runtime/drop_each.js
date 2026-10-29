// What drops each item an iterator, or an array, hasn't given, by `drop`,
// as Rust's drops an owning iterator's (ADRs 0098, 0465).
function $dropEach(drop) {
  // The rest after one throws too (#416).
  const dropped = (items) => $dropItems(items, drop);
  // Each item's own drop, which a slice it passes over is dropped by (#234).
  dropped.each = drop;
  return dropped;
}

// Each of `items` dropped by `drop`, in order, the rest still dropped after
// one throws, a destructor's panic, as Rust's drop of a slice goes on past
// a panicking item; then what was thrown, thrown on, the last of them, as a
// `finally`'s would be (#416).
function $dropItems(items, drop) {
  let threw = false;
  let thrown;
  for (const item of items) {
    try {
      drop(item);
    } catch (error) {
      threw = true;
      thrown = error;
    }
  }
  if (threw) throw thrown;
}
