//! [`crypto`](https://nodejs.org/api/crypto.html): hashes.
//!
//! ```rust,ignore
//! use node::crypto::{self, BinaryToTextEncoding};
//!
//! let hash = crypto::create_hash("md5");
//! hash.update("text");
//! let hex = hash.digest(BinaryToTextEncoding::Hex);
//! ```

use core::marker::PhantomData;

use js::JsObject;

use crate::buffer::IntoStrOrBufferSource;

/// [`Hash`](https://nodejs.org/api/crypto.html#class-hash): a digest of
/// what each [`update`](Hash::update) gives it.
#[cfg_attr(rust_js, rust_js::types = "crypto#Hash")]
pub struct Hash(PhantomData<JsObject>);

/// [`crypto.createHash(algorithm)`](https://nodejs.org/api/crypto.html#cryptocreatehashalgorithm-options):
/// a [`Hash`] of `algorithm`, `"md5"` or `"sha256"`, an OpenSSL digest's
/// name; of one OpenSSL lacks, JS's `Error`, a panic.
#[cfg_attr(rust_js, rust_js::link_name = "crypto#createHash")]
pub fn create_hash(algorithm: &str) -> &'static Hash {
    unreachable!()
}

impl Hash {
    /// [`hash.update(data)`](https://nodejs.org/api/crypto.html#hashupdatedata-inputencoding):
    /// `data` added, text as UTF-8; the hash itself. Once digested, JS's
    /// `Error`, a panic.
    #[cfg_attr(rust_js, rust_js::link_name = "update")]
    pub fn update(&self, data: impl IntoStrOrBufferSource) -> &Hash {
        unreachable!()
    }

    /// [`hash.digest(encoding)`](https://nodejs.org/api/crypto.html#hashdigestencoding):
    /// the digest of what was added, as `encoding`'s text. Only once: again,
    /// JS's `Error`, a panic.
    #[cfg_attr(rust_js, rust_js::link_name = "digest")]
    pub fn digest(&self, encoding: BinaryToTextEncoding) -> String {
        unreachable!()
    }
}

/// [`BinaryToTextEncoding`](https://nodejs.org/api/crypto.html#hashdigestencoding):
/// how bytes are written as text.
#[cfg_attr(rust_js, rust_js::types = "crypto#BinaryToTextEncoding")]
pub enum BinaryToTextEncoding {
    #[cfg_attr(rust_js, rust_js::name = "base64")]
    Base64,
    #[cfg_attr(rust_js, rust_js::name = "base64url")]
    Base64url,
    #[cfg_attr(rust_js, rust_js::name = "hex")]
    Hex,
    #[cfg_attr(rust_js, rust_js::name = "binary")]
    Binary,
}
