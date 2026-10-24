// The keys `for (const key in object)` visits, its own enumerable ones and
// its prototypes', each as the loop reaches it (ADR 0719): what
// `js::keys_in(object)` is where no `for` loop is written of it.
function* $keysIn(object) {
  for (const key in object) {
    yield key;
  }
}
