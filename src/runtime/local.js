// A thread-local whose initializer runs the program's code, or may panic:
// made where it's first used, as Rust's is, and read by calling what this
// makes (ADR 0037, amended). One whose initializer panics stays unmade, so
// its next use runs it again, as Rust's does.
function $local(init) {
  let made = false;
  let value;
  const read = () => {
    if (!made) {
      value = init();
      made = true;
    }
    return value;
  };
  // `LocalKey::set` of one not made yet: made of the cell given, its
  // initializer never run, as std's (#252); whether it was.
  read.initWith = (cell) => {
    if (made) return false;
    value = cell;
    made = true;
    return true;
  };
  return read;
}
