// `advance_by(m)` of `it`, by its dictionary, `iterator`: its own, which
// drops what it passes itself, a `Vec`'s as one slice; else std's, each
// it steps past given to `drop`. How many there weren't (#286).
function $advanceByOf(it, m, drop, iterator) {
  const given = iterator === undefined ? it : $given(it, iterator, true);
  if (Object.hasOwn(given, "advanceBy")) return given.advanceBy(m, drop);
  return $Steps.prototype.advanceBy.call($lent(given), m, drop);
}

// `it.advance_by(m)`, as std's own bodies call it: `$advanceByOf`'s,
// `Ok(())` where nothing was short, else `Err` of how many there weren't
// (ADR 0913).
function $advanceBy(it, m, drop) {
  const left = $advanceByOf(it, m, drop);
  return left === 0 ? { TAG: "Ok" } : { TAG: "Err", _0: left };
}

// A `Rev`'s `advance_by`, std's: its source's `advance_back_by`, by its
// dictionaries, `iterator` and `back`: its own, which drops what it passes
// itself, a `Vec`'s as one slice; else each `next_back`, given to `drop`.
function $revAdvanceBy(rev, m, drop, iterator, back) {
  const it = back === undefined ? rev.iter : $given(rev.iter, iterator, true, back);
  if (Object.hasOwn(it, "advanceBackBy")) return it.advanceBackBy(m);
  // An array's items it owns, a `Vec`'s, dropped as one slice.
  if (it.items !== undefined && drop !== undefined) return $advanceSlice(it, m, drop, true);
  for (; m > 0; m--) {
    const step = it.back();
    if (step.done) break;
    drop?.(step.value);
  }
  return m;
}

// A `Fuse`'s `rfold`, std's: its source's, while it has one, by its
// dictionaries, `iterator` and `back`: the crate's own `rfold` where its impl
// writes one, which takes the source and drops it, else each `next_back`,
// the source dropped by `drop` after (#224).
function $fuseRfold(fuse, init, f, iterator, back, drop) {
  if (fuse.iter == null) return init;
  const iter = $someValue(fuse.iter);
  fuse.iter = undefined;
  if (back?.rfold !== undefined) return back.rfold(iter, init, f);
  const folded = $foldBack($given(iter, iterator, true, back), init, f);
  drop?.(iter);
  return folded;
}
