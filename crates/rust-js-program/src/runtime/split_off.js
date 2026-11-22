
// `v.split_off(at)`: what's from `at`, taken out, past the end a `Vec`'s
// panic, or a `VecDeque`'s (`deque`), its `assert!`'s literal, a `&str`,
// each raised where `raisedAt` says (#2292).
function $splitOff(items, at, raisedAt, deque = false) {
  if (at > items.length) {
    throw deque ? $panic("`at` out of bounds", raisedAt) : $stringPanic(`\`at\` split index (is ${at}) should be <= len (is ${items.length})`, raisedAt);
  }
  return items.splice(at);
}
