
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
// Its `Debug` into a writer, as serde_json's `write!(f, "Error({:?}, line:
// {}, column: {})", ..)` writes it: each literal piece one `write_str`, its
// message a `String`'s `Debug`, each number its digits (#2250).
function $writeDebugJsonError(f, e) {
  f.write("Error(");
  $writeDebugStr(f, e.message);
  f.write(", line: ");
  $writeInteger(f, e.line);
  f.write(", column: ");
  $writeInteger(f, e.column);
  f.write(")");
}
function $debugJsonError(e) {
  return `Error(${$debugStr(e.message)}, line: ${e.line}, column: ${e.column})`;
}
