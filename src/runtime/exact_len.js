// `it.len()` of an iterator of the crate's whose `ExactSizeIterator` keeps
// std's: its `size_hint()`'s lower bound, which its upper must be, as std
// asserts.
function $exactLen(hint) {
  const [lower, upper] = hint;
  if (upper !== lower) {
    const shown = upper === undefined ? "None" : `Some(${upper})`;
    throw new Error(`assertion \`left == right\` failed\n  left: ${shown}\n right: Some(${lower})`);
  }
  return lower;
}
