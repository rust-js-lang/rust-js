//! How the lowering reads a row of `$stdSites`, std's sites of its panics:
//! through the row's handle, `StdRow`, the catalog's (`recognition/models/
//! rows.rs`), never by its name's text, so the catalog finds each row's
//! readers by the API (Campaign 5's step 2c, ADR 1202 amended).

use crate::recognition::models::rows::StdRow;
use rust_js_program::js::Expr;

impl StdRow {
    /// Where std raises one of its panics, the row's `at`th site,
    /// `$stdSites.<row>[at]`: the runtime's constant of the toolchain, which
    /// `bun run std` writes from std's MIR, whatever calls it (#2292).
    pub fn site(self, at: usize) -> Expr {
        Expr::std_site(self.name(), at)
    }

    /// The `at`th site of the row's `tag`, `$stdSites.clonedAt[tag][at]`,
    /// of a row by a std type's key (#2509).
    pub fn site_of(self, tag: &str, at: usize) -> Expr {
        Expr::std_site_of(self.name(), tag, at)
    }

    /// Whether `bun run std` wrote the row's `at`th site of `tag`, of the
    /// runtime's own table (`runtime::std_site_written`).
    pub fn written(self, tag: &str, at: usize) -> bool {
        rust_js_program::runtime::std_site_written(self.name(), tag, at)
    }

    /// The text of the row's `at`th site of `tag` that `bun run std` wrote
    /// (`runtime::std_site_text`).
    pub fn text(self, tag: &str, at: usize) -> Option<&'static str> {
        rust_js_program::runtime::std_site_text(self.name(), tag, at)
    }
}
