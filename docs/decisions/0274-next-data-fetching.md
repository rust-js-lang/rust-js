# 0274. Next.js's getStaticProps and getStaticPaths, typed as next types them

Status: Accepted. Extends [0192](0192-next.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's errors page is a Pages Router page built for each of React's
error codes:

```tsx
export const getStaticProps: GetStaticProps<ErrorDecoderProps> = async ({params}) => {
  // ..
  if (code && !errorCodes[code]) {
    return {notFound: true};
  }
  return {props: {content, toc, meta, errorCode: code, errorMessage}};
};

export const getStaticPaths: GetStaticPaths = async () => {
  return {paths: codes.map((code) => ({params: {errorCode: code}})), fallback: false};
};
```

The next crate had the Pages Router's router, not its data fetching.

## Decision

**A page's `getStaticProps` and `getStaticPaths` are `pub async fn`s it
exports, of the next crate's types, `next`'s own:**

- `GetStaticPropsContext<Params>`, what it's given: its `params`, an
  `Option`, `None` for a page that isn't a dynamic route's, its locale and
  preview.
- `GetStaticPropsResult<Props>`, an untagged enum (ADR 0214) of
  `StaticProps { props }` and `StaticNotFound { not_found: true }`, each
  its object, `{ props }` or `{ notFound: true }`.
- `GetStaticPathsContext`, and `GetStaticPathsResult<Params> { paths,
  fallback }`, each path a `StaticPath`, written out or
  `StaticPathParams { params }`.

```rust
pub async fn getStaticPaths(_: GetStaticPathsContext) -> GetStaticPathsResult<Params> {
    let params = |code: &str| StaticPath::Params(StaticPathParams { params: Params { code: code.to_string() } });
    GetStaticPathsResult { paths: vec![params("1"), StaticPath::Path("/codes/2".to_string())], fallback: false }
}
```

```js
export async function getStaticPaths(_) {
  const params = (code) => ({ params: { code } });
  return { paths: [params("1"), "/codes/2"], fallback: false };
}
```

Not yet: a result's `revalidate`, whose `None` would be `revalidate:
undefined` in its object, a `redirect`, and a fallback of `"blocking"`, a
string an untagged enum can't be yet (ADR 0214).

## Why

- **They're `next`'s types**, as the react crate's are `@types/react`'s.
- **It's tested**: the Next.js example builds a page for the paths it
  gives, each with its props, and none for the one not found.
