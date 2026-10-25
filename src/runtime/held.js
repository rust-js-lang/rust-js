// `it`, an iterator a variable or a temporary holds, which its owner drops
// where Rust drops it, at its scope's or its statement's end: held, so
// nothing it steps is dropped as it's done or ends early, but as it's
// closed (ADR 0570 amended, #206).
function $held(it) {
  it.hold?.();
  return it;
}
