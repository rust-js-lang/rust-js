// `collect()` into an `Option` of what the crate's own `from_iter` makes,
// or generic code's `C`'s, which may run code: std's `try_process` (#1588).
// `collect` is given a `GenericShunt` of `items` (`$tryProcessing`); what
// it makes is the `Some`, or, where a `None` ended a step, dropped by
// `drop`, and `None` given back. `boxed`: whether an item's `Some` that may
// look like `None` is boxed, and `someBoxed`, what's made's (ADR 0051).
function $tryProcessOptions(items, collect, drop, boxed = false, someBoxed = false) {
  const { made, ended } = $tryProcessing(
    items,
    (item) => item == null,
    (item) => (boxed ? $someValue(item) : item),
    collect,
  );
  if (!ended) return someBoxed ? $some(made) : made;
  drop?.(made);
  return undefined;
}

// `collect()` into a `Result` of what the crate's own `from_iter` makes, or
// generic code's `C`'s: std's `try_process` (#1588). What `collect` makes
// is the `Ok`, or, where an `Err` ended a step, dropped by `drop`, and the
// `Err` kept given back; one kept is dropped by `dropErr` where a later
// one replaces it, or where `collect` panics.
function $tryProcessResults(items, collect, drop, dropErr) {
  const { made, ended, residual } = $tryProcessing(
    items,
    (item) => item.TAG === "Err",
    (item) => item._0,
    collect,
    dropErr && ((err) => dropErr(err._0)),
  );
  if (!ended) return { TAG: "Ok", _0: made };
  drop?.(made);
  return residual;
}

// `try_process`'s own: `collect` given a `GenericShunt` of `items`, which
// steps them by their `try_fold` (`$shuntStep`, #1004) and ends a step at
// one `isResidual` says is a `None` or an `Err`, keeping it, each item's
// value `valueOf`'s; stepped on past one, it goes on, as std's does, a
// later one kept in its place, the one before dropped by `dropResidual`.
// The shunt is `collect`'s, which drops it as std's `from_iter` drops what
// it's given, by its field `iter`, as std's struct's drop glue does. Where
// `collect` throws, a panic, what's kept is dropped by `dropResidual`, a
// local of `try_process`'s frame.
function $tryProcessing(items, isResidual, valueOf, collect, dropResidual) {
  const it = items[Symbol.iterator]();
  let ended = false;
  let residual;
  const shunt = new $Steps(
    () => {
      const step = $shuntStep(it);
      if (step.done) return step;
      const item = step.value;
      if (isResidual(item)) {
        const before = residual;
        residual = item;
        if (ended) dropResidual?.(before);
        ended = true;
        return { done: true, value: undefined };
      }
      return { done: false, value: valueOf(item) };
    },
    () => it.return?.(),
  );
  shunt.iter = it;
  const made = $unwinds(
    () => collect(shunt),
    dropResidual && (() => ended && dropResidual(residual)),
  );
  return { made, ended, residual };
}
