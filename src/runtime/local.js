// A thread-local whose initializer runs the program's code, or may panic:
// made where it's first used, as Rust's is, and read by calling what this
// makes (ADR 0037, amended). One whose initializer panics stays unmade, so
// its next use runs it again, as Rust's does.
function $local(init) {
  let made = false;
  let value;
  return () => {
    if (!made) {
      value = init();
      made = true;
    }
    return value;
  };
}
