// A string's `chars()` the crate steps from its back, clones, or asks its
// rest or its `Debug`, and every `char_indices()`, `indexed`, whose items
// are `[byte, char]` (#1186): lazy as std's are, each `char` read where
// it's stepped to, from its front or its back, so `next()` and `next_back()`
// touch only what they read. Where it is are UTF-16 indices of `s`, its
// front `at` and its back `end`; of `char_indices()`, where its front is in
// UTF-8 bytes too, and its back, counted where it's first stepped. One
// stepped only forward is JS's own string iterator, `s[Symbol.iterator]()`,
// which reads `char`s as Rust's do.
function $chars(s, indexed = false) {
  const it = Object.create(Iterator.prototype);
  it.s = s;
  it.at = 0;
  it.end = s.length;
  it.indexed = indexed;
  // Where its front is in UTF-8 bytes, and how many bytes it's stepped from
  // its back, so what's left is counted by its text's length in bytes,
  // kept of a long one (#1184), never again of each part.
  it.byte = 0;
  it.backBytes = 0;
  it.next = $charsNext;
  it.back = $charsBack;
  it.rest = $charsRest;
  it.clone = $charsClone;
  it.sizeHint = $charsHint;
  return it;
}

// `char_indices()`: each `char`, with where it starts in UTF-8 bytes.
function $charIndices(s) {
  return $chars(s, true);
}

function $charsNext() {
  if (this.at >= this.end) return { value: undefined, done: true };
  const c = String.fromCodePoint(this.s.codePointAt(this.at));
  this.at += c.length;
  const at = this.byte;
  this.byte += $byteLen(c);
  return { value: this.indexed ? [at, c] : c, done: false };
}

// Its last `char`, as `next()` gives its first: what `$nextBack` steps. A
// low surrogate past the front is its pair's second half.
function $charsBack() {
  if (this.at >= this.end) return { value: undefined, done: true };
  let from = this.end - 1;
  const unit = this.s.charCodeAt(from);
  if (unit >= 0xdc00 && unit <= 0xdfff && from > this.at) from--;
  const c = this.s.slice(from, this.end);
  this.end = from;
  this.backBytes += $byteLen(c);
  return { value: this.indexed ? [$byteLen(this.s) - this.backBytes, c] : c, done: false };
}

// `as_str()`: the text of the `char`s it has left (#118), a long one's
// facts of where it is in its text's.
function $charsRest() {
  const part = this.s.slice(this.at, this.end);
  if (part.length >= $textLong) $textPart(part, this.s, this.byte, this.at, $byteLen(this.s) - this.backBytes - this.byte);
  return part;
}

function $charsClone() {
  return Object.assign(Object.create(Iterator.prototype), this);
}

// Std's hint of the bytes it has left, `bytes`, each `char` 1 to 4 of
// them: `(bytes.div_ceil(4), Some(bytes))` (str/iter.rs:100, 151).
function $charsHint() {
  const bytes = $byteLen(this.s) - this.backBytes - this.byte;
  return [Math.ceil(bytes / 4), bytes];
}

// `it.as_str()` of a string's `char_indices()`: the text of the `char`s
// it has left (#118).
function $charIndicesStr(it) {
  return it.rest();
}

// `it.offset()` of a string's `char_indices()`: where its next `char`
// starts, in UTF-8 bytes, or the string's length past its last, as std's
// front is (#118).
function $charIndicesOffset(it) {
  return it.byte;
}
