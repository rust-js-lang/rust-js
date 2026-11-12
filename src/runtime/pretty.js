
function $pretty(open, items, close, rest = false) {
  if (items.length === 0) {
    return open + (rest ? " .. " : "") + close;
  }
  const lines = items.map((item) => $padded(item + ",\n")).join("");
  return open + "\n" + lines + (rest ? "    ..\n" : "") + close;
}
