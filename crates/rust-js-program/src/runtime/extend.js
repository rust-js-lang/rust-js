// `v.extend(items)`: each pushed in turn. Where stepping them throws, a
// panic in a closure, what's left of them closed, dropped, as Rust drops
// the iterator it was given as it unwinds; what it pushed stays, `v`'s
// (#422). `byFold`, of a `TrustedLen` iterator, by its fold, as std's
// `extend_trusted` steps it, which drops what it holds where its fold
// does, a `map`'s function before what it hasn't given (#710); and of one
// `$folded` made, by its fold, as std's consumer that routes through it is
// (#1161). Of a range's or a chain of ranges', which may be longer than
// `usize::MAX`, `unbounded`, its hint asked first, as std's
// `extend_trusted` asks it: past `usize::MAX`, where std's has no upper
// bound, std's `capacity overflow` before any is pushed, where JS would
// push to its arrays' longest and throw its own (#2511).
function $extend(v, items, byFold = false, unbounded = false) {
  // A `TrustedLen` one grown as std grows it, `$grownBy`'s, as std's
  // `extend_trusted` steps it (#1803, #1805).
  if (byFold && typeof items.extendTrusted === "function") {
    items.extendTrusted((x) => {
      v.push(x);
    });
    return;
  }
  if (unbounded && !(($sizeHint(items)[1] ?? Infinity) <= 2 ** 32 - 1)) throw $panic("capacity overflow", $stdSites.vecExtendTrusted[0]);
  if ((byFold || items.byItsFold) && typeof items.fold === "function") {
    items.fold(undefined, (_, x) => {
      v.push(x);
    });
    return;
  }
  const it = items[Symbol.iterator]();
  $unwinds(
    () => {
      for (let step = it.next(); !step.done; step = it.next()) v.push(step.value);
    },
    () => it.return?.(),
  );
}

