//! [`next/web-vitals`](https://nextjs.org/docs/app/api-reference/functions/use-report-web-vitals):
//! the page's [Web Vitals](https://web.dev/articles/vitals), as each is
//! measured.

use core::marker::PhantomData;

use js::JsObject;
use react::webapi::PerformanceEntry;

/// Call `report_web_vitals_fn` with each metric as it's measured, as
/// `useReportWebVitals(fn)`: a client component's.
#[cfg_attr(rust_js, rust_js::link_name = "next/web-vitals#useReportWebVitals")]
pub fn use_report_web_vitals(report_web_vitals_fn: impl Fn(&Metric) + 'static) {
    unreachable!()
}

/// A Web Vital, as the `web-vitals` package Next.js reports with types it.
pub struct Metric(PhantomData<JsObject>);

impl Metric {
    /// Which: `"CLS"`, `"FCP"`, `"FID"`, `"INP"`, `"LCP"` or `"TTFB"`.
    #[cfg_attr(rust_js, rust_js::link_name = "get name")]
    pub fn name(&self) -> &'static str {
        unreachable!()
    }

    /// Its value: milliseconds, or for `CLS`, a score.
    #[cfg_attr(rust_js, rust_js::link_name = "get value")]
    pub fn value(&self) -> f64 {
        unreachable!()
    }

    /// `"good"`, `"needs-improvement"` or `"poor"`, by its thresholds.
    #[cfg_attr(rust_js, rust_js::link_name = "get rating")]
    pub fn rating(&self) -> &'static str {
        unreachable!()
    }

    /// How much it changed since it was last reported.
    #[cfg_attr(rust_js, rust_js::link_name = "get delta")]
    pub fn delta(&self) -> f64 {
        unreachable!()
    }

    /// Its id, the same for each report of it in one page load.
    #[cfg_attr(rust_js, rust_js::link_name = "get id")]
    pub fn id(&self) -> &'static str {
        unreachable!()
    }

    /// The performance entries it's of.
    #[cfg_attr(rust_js, rust_js::link_name = "get entries")]
    pub fn entries(&self) -> Vec<&'static PerformanceEntry> {
        unreachable!()
    }

    /// How the page was loaded: `"navigate"`, `"reload"`, `"back-forward"`,
    /// `"back-forward-cache"`, `"prerender"` or `"restore"`.
    #[cfg_attr(rust_js, rust_js::link_name = "get navigationType")]
    pub fn navigation_type(&self) -> &'static str {
        unreachable!()
    }
}
