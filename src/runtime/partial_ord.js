// A `PartialEq` dictionary (ADRs 0049, 0053): its `eq`, and `ne`, an impl's
// own where it writes one, or std's that asks its parts' `ne`, a tuple's,
// else std's provided `!eq` (#962). The one place std's default is.
function $partialEq(dictionary) {
  const eq = dictionary.eq;
  dictionary.ne ??= (a, b) => !eq(a, b);
  return dictionary;
}

// A `PartialOrd` dictionary (ADRs 0049, 0057): its `PartialEq()`, its
// `partialCmp`, and `lt`, `le`, `gt` and `ge`, an impl's own where it writes
// them, else std's provided bodies over its `partialCmp`, which `None`'s
// `undefined` makes false (#937, #944). The one place std's defaults are.
function $partialOrd(dictionary) {
  const partialCmp = dictionary.partialCmp;
  dictionary.lt ??= (a, b) => partialCmp(a, b) < 0;
  dictionary.le ??= (a, b) => partialCmp(a, b) <= 0;
  dictionary.gt ??= (a, b) => partialCmp(a, b) > 0;
  dictionary.ge ??= (a, b) => partialCmp(a, b) >= 0;
  return dictionary;
}

// An `Ord` dictionary (ADRs 0049, 0057): its `PartialOrd()`, its `cmp`, and
// `max`, `min` and `clamp`, an impl's own where it writes them, else std's
// provided bodies over its `PartialOrd()`'s operators, `other < self`, and
// `min <= max`, `self < min` and `self > max` (#962). Generic code gives
// each, last, the drop of a type with a destructor, by which std's drop
// the ones they don't give back (#912); an impl's own drops its own. The
// one place std's defaults are.
function $ord(dictionary) {
  const partialOrd = dictionary.PartialOrd;
  dictionary.max ??= (a, b, drop) => $maxOf(a, b, partialOrd(), drop);
  dictionary.min ??= (a, b, drop) => $minOf(a, b, partialOrd(), drop);
  dictionary.clamp ??= (x, min, max, drop) => $clampBy(x, min, max, partialOrd(), undefined, drop);
  return dictionary;
}

// std's `PartialEq`, `PartialOrd` and `Ord` of what `$cmp` orders as Rust
// does and nothing sees, numbers but floats, text, `char`s and `bool`s: one
// shared dictionary each, `===` their `PartialEq` (#944). An integer's and
// a `char`'s `Ord` write their own `clamp`, whose panic shows the bounds as
// `{:?}` does, `$intsOrdered`'s and `$charsOrdered`'s; the rest keep std's
// (#962).
const $strictlyEqual = /* @__PURE__ */ $partialEq({ eq: (a, b) => a === b });

const $partiallyOrdered = /* @__PURE__ */ $partialOrd({
  PartialEq: () => $strictlyEqual,
  partialCmp: $cmp,
});

const $ordered = /* @__PURE__ */ $ord({ PartialOrd: () => $partiallyOrdered, cmp: $cmp });

const $intsOrdered = /* @__PURE__ */ $ord({ PartialOrd: () => $partiallyOrdered, cmp: $cmp, clamp: $clamp });

const $charsOrdered = /* @__PURE__ */ $ord({
  PartialOrd: () => $partiallyOrdered,
  cmp: $cmp,
  clamp: (x, min, max) => $clampBy(x, min, max, $partiallyOrdered, (c) => $debugStr(c, "'")),
});
