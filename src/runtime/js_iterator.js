// The `Iterator` dictionary generic code is given of a JS iterator, its
// steps the runtime's, which ask what the iterator has of its own where it
// has it, else step it as std's defaults do: one named object, so each
// chain that's given one shares it, `$jsIteratorSome` of items that look
// like `None`, boxed (ADR 0051). A dictionary any entry of which would
// differ stays its own (`iterator_dictionary`, #1681). Pure, so a bundle
// that asks neither leaves both out, and what they name with them.
const $jsIterator = /* @__PURE__ */ Object.freeze({ next: $next, fold: $foldOf, nth: $nth, last: $last, count: $count });
const $jsIteratorSome = /* @__PURE__ */ Object.freeze({
  next: $nextSome,
  fold: $foldOf,
  nth: $nthSome,
  last: $lastSome,
  count: $count,
});
