// What a closure carries its clone under (ADR 0522).
const $cloneKey = Symbol("rust-js.clone");

// A closure cloned apart, carrying its clone, `clone`, which makes it again
// of a clone of what it took (ADR 0522).
function $cloning(f, clone) {
  f[$cloneKey] = clone;
  return f;
}

// A clone of a closure cloned apart: the one it carries, `$cloning`'s, made.
function $clone(f) {
  return f[$cloneKey]();
}
