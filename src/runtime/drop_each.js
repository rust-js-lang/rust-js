// What drops each item an iterator, or an array, hasn't given, by `drop`,
// as Rust's drops an owning iterator's (ADRs 0098, 0465).
function $dropEach(drop) {
  const dropped = (items) => {
    for (const item of items) drop(item);
  };
  // Each item's own drop, which a slice it passes over is dropped by (#234).
  dropped.each = drop;
  return dropped;
}
