
function $round(x) {
  return Math.sign(x) * Math.round(Math.abs(x));
}

// `x.round_ties_even()`: the nearest integer, a tie to the even one. JS's
// `Math.round` takes a tie up, so an odd one it reached from a tie goes
// down instead; `-0.5`'s is `-0`, as Rust's is.
function $roundTiesEven(x) {
  const r = Math.round(x);
  return r - x === 0.5 && r % 2 !== 0 ? r - 1 : r;
}
