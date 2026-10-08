
## Since

- **`if x.is_none() { x = Some(e) }` is `x ??= e`**: the same test, and `e`
  made only where `x` is none, as react.dev's errors page caches the codes
  it fetched, `cachedErrorCodes ||= ..`.
