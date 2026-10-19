// What drops each item an iterator, or an array, hasn't given, by `drop`,
// as Rust's drops an owning iterator's (ADRs 0098, 0465).
function $dropEach(drop) {
  return (items) => {
    for (const item of items) drop(item);
  };
}
