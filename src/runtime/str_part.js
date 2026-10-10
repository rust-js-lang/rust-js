// `&mut s[start..end]` of the string a cell holds (ADR 0334): checked as
// `&s[start..end]` is, then that part of it, read and written in place.
// Nothing a `&mut str` does changes its length in bytes, so the part stays
// where it was.
function $strPart(cell, start, end) {
  end ??= $byteLen(cell.value);
  $strSlice(cell.value, start, end);
  return {
    get value() {
      return $strSlice(cell.value, start, end);
    },
    set value(text) {
      cell.value = $strSlice(cell.value, 0, start) + text + $strSlice(cell.value, end);
    },
  };
}

// `s.get_mut(start..end)`: that part, or `undefined`, `None`, where
// `&mut s[start..end]` would panic.
function $strGetMut(cell, start, end) {
  return $strGet(cell.value, start, end) === undefined ? undefined : $strPart(cell, start, end);
}

// `s.split_at_mut(at)`: the parts before and after the byte `at`, panicking
// as `&s[..at]` does; or, `checked`, `undefined` where that would panic.
function $strSplitAtMut(cell, at, checked) {
  if (checked && $charBoundary(cell.value, at) === undefined) return undefined;
  return [$strPart(cell, 0, at), $strPart(cell, at)];
}
