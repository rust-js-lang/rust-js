// `s.splitn(n, p)`: its first `n - 1` pieces, and what's left, whole.
function $splitN(s, n, p) {
  if (n === 0) return [];
  const parts = $split(s, p);
  return parts.length <= n ? parts : [...parts.slice(0, n - 1), parts.slice(n - 1).join(p)];
}
