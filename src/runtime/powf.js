// `a.powf(b)`, as Rust's is, IEEE's `pow`: 1 of a base of 1 to any power, a
// NaN's too, and of -1 to an infinite one, where JS's `**` gives NaN.
function $powf(a, b) {
  return a === 1 || (a === -1 && Math.abs(b) === Infinity) ? 1 : a ** b;
}
