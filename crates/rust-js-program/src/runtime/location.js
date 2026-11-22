// A `Location` (ADR 0427, amended by #2292): the text `file:line:column`,
// where rustc places a call, each panic's as each `Location::caller()`'s,
// one compact form. Read from its end, as a file may hold a `:`, a
// Windows path's `C:\`; equal, and hashed, as its parts are, as text is.
function $locationParts(location) {
  const column = location.lastIndexOf(":");
  const line = location.lastIndexOf(":", column - 1);
  return [location.slice(0, line), Number(location.slice(line + 1, column)), Number(location.slice(column + 1))];
}
// `location.file()`.
function $locationFile(location) {
  return $locationParts(location)[0];
}
// `location.line()`.
function $locationLine(location) {
  return $locationParts(location)[1];
}
// `location.column()`.
function $locationColumn(location) {
  return $locationParts(location)[2];
}
// `{:?}` of a `Location`, std's: `Location { file: "m.rs", line: 9, column: 13 }`.
function $locationDebug(location) {
  const [file, line, column] = $locationParts(location);
  return `Location { file: ${$debugStr(file)}, line: ${line}, column: ${column} }`;
}
// `a.cmp(&b)` of two `Location`s, as std derives it: by file, then line,
// then column, each as its own type orders, not as their text does.
function $locationCmp(a, b) {
  const [x, y] = [$locationParts(a), $locationParts(b)];
  for (let at = 0; at < 3; at++) {
    if (x[at] < y[at]) return -1;
    if (x[at] > y[at]) return 1;
  }
  return 0;
}
