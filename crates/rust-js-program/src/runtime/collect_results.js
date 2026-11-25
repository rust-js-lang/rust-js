// `collect()` into a `Result`: the first `Err`, where it stops, as Rust's
// does, or `Ok` of all the values, made `into` the collection where it's
// one but an array. Stopped, what it steps is closed, dropping what's left
// of it, then each value collected is dropped by `drop`, as std's are, and
// as the close unwinds, then the `Err` kept, by `dropErr` (#2564).
// Where stepping it throws, a panic in a closure, what it collected and
// what's left are dropped as std's unwinds, what's left first,
// `restFirst`, where it doesn't collect in place (#423). Of a `Vec` of
// values of `size` bytes, where its hint may run the crate's own, that
// hint asked as std's `Vec` grows of the `GenericShunt` (`$shuntGrowth`):
// where asking it panics, the value just taken dropped first (#848); the
// capacity it reaches kept (#1180).
function $collectResults(items, into, drop, restFirst = false, size, dropErr) {
  const values = [];
  let failed;
  const it = items[Symbol.iterator]();
  const grown = size === undefined ? undefined : $shuntGrowth(it, size);
  $collectUnwinding(() => {
    // Each step `GenericShunt`'s, by its source's `try_fold` (#1004).
    for (let step = $shuntStep(it); !step.done; step = $shuntStep(it)) {
      const item = step.value;
      if (item.TAG === "Err") {
        failed = item;
        break;
      }
      $unwinds(
        () => grown?.(values.length),
        () => drop?.(item._0),
      );
      values.push(item._0);
    }
  }, it, values, drop, restFirst);
  grown?.keep(values);
  if (failed === undefined) return { TAG: "Ok", _0: into ? into(values) : values };
  // Closed, as std's `from_iter` drops the `GenericShunt` it's given before
  // the `Vec` it made: where that close panics, what it collected is
  // dropped as the panic unwinds `from_iter`'s frame, then the `Err` kept,
  // by `dropErr`, a local of `try_process`'s (#2564).
  $closeHolding(
    it,
    drop && (() => $dropItems(values, drop)),
    dropErr && (() => dropErr(failed._0)),
  );
  if (drop) $dropItems(values, drop);
  return failed;
}
