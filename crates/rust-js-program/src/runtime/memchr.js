// memchr's `memchr(n, haystack)`, `memchr2` and `memchr3` (ADR 0529): where
// in `haystack`, an array of bytes, one of `needles` is first, or
// `undefined`, `None`: an index of the slice it's given. Of one needle,
// JS's own search, `indexOf`.
function $memchr(haystack, needles) {
  if (needles.length === 1) {
    const at = haystack.indexOf(needles[0]);
    return at < 0 ? undefined : at;
  }
  for (let i = 0; i < haystack.length; i++) {
    if (needles.includes(haystack[i])) return i;
  }
  return undefined;
}

// memchr's `memrchr`, `memrchr2` and `memrchr3`: the last one.
function $memrchr(haystack, needles) {
  if (needles.length === 1) {
    const at = haystack.lastIndexOf(needles[0]);
    return at < 0 ? undefined : at;
  }
  for (let i = haystack.length - 1; i >= 0; i--) {
    if (needles.includes(haystack[i])) return i;
  }
  return undefined;
}

// memchr's `memchr_iter`, `memchr2_iter` and `memchr3_iter`: each index of
// one of `needles` in `haystack`, from either end, which meet, as memchr's
// `Memchr` steps them; and a clone of it, where it is (ADR 0525).
function $memchrIter(haystack, needles, at = 0, end = haystack.length) {
  const one = needles.length === 1 ? needles[0] : undefined;
  const found = (i) => (one === undefined ? needles.includes(haystack[i]) : haystack[i] === one);
  const it = Iterator.from({
    next: () => {
      if (one !== undefined) {
        const i = haystack.indexOf(one, it.at);
        if (i < 0 || i >= it.end) {
          it.at = it.end;
          return { done: true, value: undefined };
        }
        it.at = i + 1;
        return { done: false, value: i };
      }
      while (it.at < it.end) {
        const i = it.at++;
        if (found(i)) return { done: false, value: i };
      }
      return { done: true, value: undefined };
    },
  });
  it.at = at;
  it.end = end;
  it.back = () => {
    while (it.at < it.end) {
      const i = --it.end;
      if (found(i)) return { done: false, value: i };
    }
    return { done: true, value: undefined };
  };
  it[$cloneKey] = () => $memchrIter(haystack, needles, it.at, it.end);
  return it;
}
