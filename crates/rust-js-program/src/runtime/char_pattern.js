// A `str`'s splits and searches of a predicate of a `char`, a closure, a
// function or a set of `char`s (ADR 0157), as std's `CharPredicateSearcher`
// searches: each `char` asked once, from the front, `next`, or from the
// back, `back`, the two meeting where they've each asked, so a closure that
// prints or panics is asked as Rust asks it (#1501). A match is its `char`'s
// index; `unit` and `byte` say where each `char` starts in UTF-16 units and
// in UTF-8 bytes (#118). -1 is none.
function $charSearcher(s, test) {
  const chars = Array.from(s);
  const unit = [0];
  const byte = [0];
  for (const c of chars) {
    unit.push(unit[unit.length - 1] + c.length);
    byte.push(byte[byte.length - 1] + $byteLen(c));
  }
  let [front, back] = [0, chars.length];
  const next = () => {
    while (front < back) {
      const k = front++;
      if (test(chars[k])) return k;
    }
    return -1;
  };
  const fromBack = () => {
    while (back > front) {
      const k = --back;
      if (test(chars[k])) return k;
    }
    return -1;
  };
  return { chars, unit, byte, next, back: fromBack };
}

// Std's `SplitInternal` of `s` by `test`: what's between its matches, each
// `next` from the front and `nextBack` from the back, `undefined` past its
// end, a trailing empty piece given where `allowTrailingEmpty`, and its
// inclusive steps, each piece with the `char` that ends it (str/iter.rs).
function $splitInternal(s, test, allowTrailingEmpty) {
  const searcher = $charSearcher(s, test);
  const at = { start: 0, end: searcher.chars.length, allowTrailingEmpty, finished: false };
  const piece = (a, b) => s.slice(searcher.unit[a], searcher.unit[b]);
  const getEnd = () => {
    if (at.finished) return undefined;
    // Flags, not a state: std's `SplitInternal`'s `finished`, as it sets it.
    at.finished = true;
    return at.allowTrailingEmpty || at.end > at.start ? piece(at.start, at.end) : undefined;
  };
  const next = (inclusive) => {
    if (at.finished) return undefined;
    const k = searcher.next();
    if (k < 0) return getEnd();
    const elt = piece(at.start, inclusive ? k + 1 : k);
    at.start = k + 1;
    return elt;
  };
  const nextBack = (inclusive) => {
    if (at.finished) return undefined;
    if (!at.allowTrailingEmpty) {
      // Flags, not a state: std's `allow_trailing_empty`, as it sets it.
      at.allowTrailingEmpty = true;
      const elt = nextBack(inclusive);
      if (elt !== undefined && elt !== "") return elt;
      if (at.finished) return undefined;
    }
    const k = searcher.back();
    if (k < 0) {
      // Flags, not a state: std's `SplitInternal`'s `finished`, as it sets it.
      at.finished = true;
      return piece(at.start, at.end);
    }
    const elt = piece(k + 1, at.end);
    at.end = inclusive ? k + 1 : k;
    return elt;
  };
  return { getEnd, next, nextBack };
}

// A step of one of these: what it gives, or its end.
function $searchStep(value) {
  return value === undefined ? { done: true, value: undefined } : { done: false, value };
}

// `s.split(p)`, or `s.splitn(n, p)`, of a predicate that may run the
// program's code: std's `Split`, stepped from either end, or `SplitN`, the
// last of its `n` what's left whole, each piece asked as it's stepped.
function $lazySplitBy(s, test, n = Infinity) {
  const split = $splitInternal(s, test, true);
  let count = n;
  const it = new $Steps(() => {
    if (count === Infinity) return $searchStep(split.next(false));
    if (count === 0) return $searchStep(undefined);
    count--;
    return $searchStep(count === 0 ? split.getEnd() : split.next(false));
  });
  if (n === Infinity) it.back = () => $searchStep(split.nextBack(false));
  return it;
}

// `s.rsplit(p)`, or `s.rsplitn(n, p)`: std's `RSplit`, `Split` stepped from
// its back, or `RSplitN`, the last of its `n` what's left whole.
function $lazyRsplitBy(s, test, n = Infinity) {
  const split = $splitInternal(s, test, true);
  let count = n;
  const it = new $Steps(() => {
    if (count === Infinity) return $searchStep(split.nextBack(false));
    if (count === 0) return $searchStep(undefined);
    count--;
    return $searchStep(count === 0 ? split.getEnd() : split.nextBack(false));
  });
  if (n === Infinity) it.back = () => $searchStep(split.next(false));
  return it;
}

// `s.split_terminator(p)`: std's `SplitTerminator`, no empty piece at its
// end; `rsplit_terminator`'s, `rev`, stepped from its back.
function $lazySplitTerminatorBy(s, test, rev = false) {
  const split = $splitInternal(s, test, false);
  const [front, back] = rev ? [split.nextBack, split.next] : [split.next, split.nextBack];
  const it = new $Steps(() => $searchStep(front(false)));
  it.back = () => $searchStep(back(false));
  return it;
}

// `s.split_inclusive(p)`: std's `SplitInclusive`, each piece with the `char`
// that ends it, none empty at its end.
function $lazySplitInclusiveBy(s, test) {
  const split = $splitInternal(s, test, false);
  const it = new $Steps(() => $searchStep(split.next(true)));
  it.back = () => $searchStep(split.nextBack(true));
  return it;
}

// `s.matches(p)`: each `char` it holds of, from the front; `rmatches`',
// `rev`, from the back. Of `match_indices`, `indices`, each with where it
// starts in UTF-8 bytes. Each asked as it's stepped, from either end.
function $lazyMatchesBy(s, test, rev = false, indices = false) {
  const searcher = $charSearcher(s, test);
  const found = (k) => (k < 0 ? undefined : indices ? [searcher.byte[k], searcher.chars[k]] : searcher.chars[k]);
  const [front, back] = rev ? [searcher.back, searcher.next] : [searcher.next, searcher.back];
  const it = new $Steps(() => $searchStep(found(front())));
  it.back = () => $searchStep(found(back()));
  return it;
}

// `s.match_indices(p)`: `$lazyMatchesBy`'s, each with where it starts in
// UTF-8 bytes; `rmatch_indices`', `rev`, from the back.
function $lazyMatchIndicesBy(s, test, rev = false) {
  return $lazyMatchesBy(s, test, rev, true);
}

// `s.rsplit(p)`, or `s.rsplitn(n, p)`: its pieces from the end, of
// `rsplitn` at most `n`, the last what's left of it whole.
function $rsplitBy(s, test, n = Infinity) {
  return Array.from($lazyRsplitBy(s, test, n));
}

// `s.split_terminator(p)`: `split`'s pieces without the empty one its end
// leaves; `rsplit_terminator`'s, `rev`, from the end.
function $splitTerminatorBy(s, test, rev = false) {
  return Array.from($lazySplitTerminatorBy(s, test, rev));
}

// `s.split_inclusive(p)`: each piece with the `char` that ends it, and none
// empty at the end.
function $splitInclusiveBy(s, test) {
  return Array.from($lazySplitInclusiveBy(s, test));
}

// `s.match_indices(p)`: each `char` it holds of, and where it starts in
// UTF-8 bytes; `rmatch_indices`', `rev`, from the end.
function $matchIndicesBy(s, test, rev = false) {
  return Array.from($lazyMatchIndicesBy(s, test, rev));
}

// `s.split_once(p)`: what's before its first match and after it, or
// `None`; `rsplit_once`'s, `last`, of its last. Searched as std's `find`
// and `rfind` search: `p` asked of each `char` from its front, or from its
// back, until it holds, never of one past it (#1501).
function $splitOnceBy(s, test, last = false) {
  const split = (unit, c) => [s.slice(0, unit), s.slice(unit + c.length)];
  if (!last) {
    let unit = 0;
    for (const c of s) {
      if (test(c)) return split(unit, c);
      unit += c.length;
    }
    return undefined;
  }
  const chars = Array.from(s);
  let unit = s.length;
  for (let k = chars.length - 1; k >= 0; k--) {
    unit -= chars[k].length;
    if (test(chars[k])) return split(unit, chars[k]);
  }
  return undefined;
}

// `s.strip_prefix(p)`: what's after its first `char`, where `p` holds of it,
// or `None`; `strip_suffix`'s, `suffix`, before its last.
function $stripBy(s, test, suffix = false) {
  const chars = Array.from(s);
  const c = suffix ? chars[chars.length - 1] : chars[0];
  if (c === undefined || !test(c)) return undefined;
  return suffix ? s.slice(0, s.length - c.length) : s.slice(c.length);
}

// `s.replace(p, to)`: each `char` it holds of, `to`; `replacen`'s, the first
// `n` of them.
function $replaceBy(s, test, to, n = Infinity) {
  let out = "";
  let count = 0;
  for (const c of s) {
    if (count < n && test(c)) {
      out += to;
      count++;
    } else {
      out += c;
    }
  }
  return out;
}
