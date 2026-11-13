
function $displayJsonError(e) {
  return e.line === 0 ? e.message : `${e.message} at line ${e.line} column ${e.column}`;
}
// Its `Display` into a writer, as serde_json's writes it: its code's message
// one `write_str`, then where it has a line, `write!`'s pieces of it and its
// column (#1652).
function $writeJsonError(f, e) {
  f.write(e.message);
  if (e.line === 0) return;
  f.write(" at line ");
  $writeInteger(f, e.line);
  f.write(" column ");
  $writeInteger(f, e.column);
}
function $debugJsonError(e) {
  return `Error(${$debugStr(e.message)}, line: ${e.line}, column: ${e.column})`;
}
