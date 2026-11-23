// `advance_by(m)` of `it`, by its dictionary, `iterator`: its own, which
// drops what it passes itself, a `Vec`'s as one slice; else std's, each
// it steps past given to `drop`. How many there weren't (#286).
function $advanceByOf(it, m, drop, iterator) {
  const given = iterator === undefined ? it : $given(it, iterator, true);
  if ($ownMethod(given, "advanceBy")) return given.advanceBy(m, drop);
  return $Steps.prototype.advanceBy.call($lent(given), m, drop);
}

// `it.advance_by(m)`, as std's own bodies call it: `$advanceByOf`'s,
// `Ok(())` where nothing was short, else `Err` of how many there weren't
// (ADR 0913).
function $advanceBy(it, m, drop) {
  const left = $advanceByOf(it, m, drop);
  return left === 0 ? { TAG: "Ok" } : { TAG: "Err", _0: left };
}
