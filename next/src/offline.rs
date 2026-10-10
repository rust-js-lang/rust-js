//! [`next/offline`](https://nextjs.org/docs/app/api-reference/functions/use-offline):
//! whether the app is offline.

/// Whether the app is offline, as `useOffline()`: `false` while it's
/// rendered on the server and hydrated.
#[cfg_attr(rust_js, rust_js::link_name = "next/offline#useOffline")]
pub fn use_offline() -> bool {
    unreachable!()
}
