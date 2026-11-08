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

// std's `PartialOrd` and `Ord` of what `$cmp` orders as Rust does and
// nothing sees, numbers but floats, text, `char`s and `bool`s: one shared
// dictionary each, `===` their `PartialEq` (#944).
const $partiallyOrdered = /* @__PURE__ */ $partialOrd({
  PartialEq: () => ({ eq: (a, b) => a === b }),
  partialCmp: $cmp,
});

const $ordered = { PartialOrd: () => $partiallyOrdered, cmp: $cmp };
