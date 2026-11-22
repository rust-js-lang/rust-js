// The text of the `char`s an iterator has left: a string's `chars()`'
// own, `$chars`' (#1186), or of a `$iter` of `char`s, a case mapping's or
// a text escape's.
function $restStr(it) {
  return it.rest !== undefined ? it.rest() : it.items.slice(it.at, it.end).join("");
}
