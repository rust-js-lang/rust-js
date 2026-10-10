//! [`next.config.js`](https://nextjs.org/docs/app/api-reference/config/next-config-js):
//! the app's config, `NextConfig`, as Next.js types it: what a function
//! the module exports by default gives, of the phase it's for.
//!
//! ```rust,ignore
//! pub fn nextConfig(phase: &str) -> NextConfig<'static> {
//!     NextConfig { output: Some(Output::Export), ..Default::default() }
//! }
//!
//! js::export_default!(nextConfig);
//! ```

use js::{Dict, Json, Promise, RegExp, Unknown};
use react::webapi::URL;

use crate::{QueryValue, SizeLimit};

/// The app's config, each `None` but what's given, as `NextConfig` types it.
#[derive(Default)]
pub struct NextConfig<'a> {
    /// The origins, besides the server's own, a development server takes
    /// requests from: `"local-origin.dev"`, `"*.local-origin.dev"`.
    #[cfg_attr(rust_js, rust_js::name = "allowedDevOrigins")]
    pub allowed_dev_origins: Option<&'a [&'a str]>,
    /// The paths a static export writes, of the default map Next.js makes.
    #[cfg_attr(rust_js, rust_js::name = "exportPathMap")]
    pub export_path_map: Option<Box<dyn Fn(&'static Dict<ExportPathMapEntry<'static>>, ExportPathMapContext) -> Promise<&'static Dict<ExportPathMapEntry<'static>>>>>,
    /// Internationalized routing, the Pages Router's.
    pub i18n: Option<I18NConfig<'a>>,
    pub typescript: Option<TypeScriptConfig<'a>>,
    /// Typed links, `Route`'s of each route.
    #[cfg_attr(rust_js, rust_js::name = "typedRoutes")]
    pub typed_routes: Option<bool>,
    /// The headers of each path's response.
    pub headers: Option<Box<dyn Fn() -> Promise<Vec<Header<'static>>>>>,
    /// The paths each served as another's.
    pub rewrites: Option<Box<dyn Fn() -> Promise<Rewrites<'static>>>>,
    /// The paths each sent to another.
    pub redirects: Option<Box<dyn Fn() -> Promise<Vec<Redirect<'static>>>>>,
    #[cfg_attr(rust_js, rust_js::name = "excludeDefaultMomentLocales")]
    pub exclude_default_moment_locales: Option<bool>,
    /// webpack's config, as Next.js gives it: `config`, changed, or another.
    pub webpack: Option<Box<dyn Fn(&'static Unknown, &'static WebpackConfigContext) -> &'static Unknown>>,
    /// Each path with a trailing slash, `/about/`, the others redirected.
    #[cfg_attr(rust_js, rust_js::name = "trailingSlash")]
    pub trailing_slash: Option<bool>,
    /// Environment variables, inlined in the app's JS.
    pub env: Option<&'a Dict<Option<&'a str>>>,
    /// Its build's directory: `.next`.
    #[cfg_attr(rust_js, rust_js::name = "distDir")]
    pub dist_dir: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "cleanDistDir")]
    pub clean_dist_dir: Option<bool>,
    /// A CDN's URL its assets are served from.
    #[cfg_attr(rust_js, rust_js::name = "assetPrefix")]
    pub asset_prefix: Option<&'a str>,
    /// The module of its cache's handler.
    #[cfg_attr(rust_js, rust_js::name = "cacheHandler")]
    pub cache_handler: Option<&'a str>,
    /// The module of its deployment's adapter.
    #[cfg_attr(rust_js, rust_js::name = "adapterPath")]
    pub adapter_path: Option<&'a str>,
    /// The module of each `"use cache"` handler, by its name: `default`,
    /// `remote`, `static`, or another.
    #[cfg_attr(rust_js, rust_js::name = "cacheHandlers")]
    pub cache_handlers: Option<&'a Dict<Option<&'a str>>>,
    /// Its cache's size in memory, in bytes; `0`, none.
    #[cfg_attr(rust_js, rust_js::name = "cacheMaxMemorySize")]
    pub cache_max_memory_size: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "useFileSystemPublicRoutes")]
    pub use_file_system_public_routes: Option<bool>,
    /// Its build's ID, `None` Next.js's own.
    #[cfg_attr(rust_js, rust_js::name = "generateBuildId")]
    pub generate_build_id: Option<Box<dyn Fn() -> Promise<Option<String>>>>,
    #[cfg_attr(rust_js, rust_js::name = "generateEtags")]
    pub generate_etags: Option<bool>,
    /// The extensions a page's file has: `["tsx", "ts", "jsx", "js"]`.
    #[cfg_attr(rust_js, rust_js::name = "pageExtensions")]
    pub page_extensions: Option<&'a [&'a str]>,
    /// The modules the client's instrumentation imports first.
    #[cfg_attr(rust_js, rust_js::name = "instrumentationClientInject")]
    pub instrumentation_client_inject: Option<&'a [&'a str]>,
    /// Responses gzipped.
    pub compress: Option<bool>,
    /// The `x-powered-by` header.
    #[cfg_attr(rust_js, rust_js::name = "poweredByHeader")]
    pub powered_by_header: Option<bool>,
    /// next/image's config.
    pub images: Option<ImageConfig<'a>>,
    /// The development server's indicator, `false` none.
    #[cfg_attr(rust_js, rust_js::name = "devIndicators")]
    pub dev_indicators: Option<DevIndicators>,
    /// How long a development server keeps a page it isn't asked for.
    #[cfg_attr(rust_js, rust_js::name = "onDemandEntries")]
    pub on_demand_entries: Option<OnDemandEntries>,
    /// Its deployment's ID, of version skew protection.
    #[cfg_attr(rust_js, rust_js::name = "deploymentId")]
    pub deployment_id: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "supportsImmutableAssets")]
    pub supports_immutable_assets: Option<bool>,
    /// The path the app is below: `"/docs"`.
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<&'a str>,
    /// Sass's options, its `implementation` among them.
    #[cfg_attr(rust_js, rust_js::name = "sassOptions")]
    pub sass_options: Option<&'a Dict<Json<'a>>>,
    #[cfg_attr(rust_js, rust_js::name = "productionBrowserSourceMaps")]
    pub production_browser_source_maps: Option<bool>,
    /// The React Compiler, on, or of its options.
    #[cfg_attr(rust_js, rust_js::name = "reactCompiler")]
    pub react_compiler: Option<ReactCompiler>,
    #[cfg_attr(rust_js, rust_js::name = "reactProductionProfiling")]
    pub react_production_profiling: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "reactStrictMode")]
    pub react_strict_mode: Option<bool>,
    /// The longest a response's headers React writes may be.
    #[cfg_attr(rust_js, rust_js::name = "reactMaxHeadersLength")]
    pub react_max_headers_length: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "httpAgentOptions")]
    pub http_agent_options: Option<HttpAgentOptions>,
    /// How long a static page's build may take, in seconds.
    #[cfg_attr(rust_js, rust_js::name = "staticPageGenerationTimeout")]
    pub static_page_generation_timeout: Option<f64>,
    /// The `crossorigin` of the `<script>`s Next.js writes.
    #[cfg_attr(rust_js, rust_js::name = "crossOrigin")]
    pub cross_origin: Option<CrossOrigin>,
    /// Its compiler's transforms.
    pub compiler: Option<CompilerConfig<'a>>,
    /// A standalone server, or a static export.
    pub output: Option<Output>,
    /// The packages Next.js compiles, as its own.
    #[cfg_attr(rust_js, rust_js::name = "transpilePackages")]
    pub transpile_packages: Option<&'a [&'a str]>,
    pub turbopack: Option<TurbopackOptions<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "skipMiddlewareUrlNormalize")]
    pub skip_middleware_url_normalize: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "skipProxyUrlNormalize")]
    pub skip_proxy_url_normalize: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "skipTrailingSlashRedirect")]
    pub skip_trailing_slash_redirect: Option<bool>,
    /// Each package's imports, written as imports of its modules.
    #[cfg_attr(rust_js, rust_js::name = "modularizeImports")]
    pub modularize_imports: Option<&'a Dict<ModularizeImport<'a>>>,
    /// What a development server logs, `false` none.
    pub logging: Option<Logging<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "enablePrerenderSourceMaps")]
    pub enable_prerender_source_maps: Option<bool>,
    /// Cache Components: `"use cache"`, and Partial Prerendering.
    #[cfg_attr(rust_js, rust_js::name = "cacheComponents")]
    pub cache_components: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "partialPrefetching")]
    pub partial_prefetching: Option<bool>,
    /// `cacheLife`'s profiles, by their names.
    #[cfg_attr(rust_js, rust_js::name = "cacheLife")]
    pub cache_life: Option<&'a Dict<CacheLifeProfile>>,
    /// How long a page's cache lasts, in seconds, of its `Cache-Control`.
    #[cfg_attr(rust_js, rust_js::name = "expireTime")]
    pub expire_time: Option<f64>,
    /// The rules Next.js writes for coding agents.
    #[cfg_attr(rust_js, rust_js::name = "agentRules")]
    pub agent_rules: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "bundlePagesRouterDependencies")]
    pub bundle_pages_router_dependencies: Option<bool>,
    /// The packages the server requires, not bundled.
    #[cfg_attr(rust_js, rust_js::name = "serverExternalPackages")]
    pub server_external_packages: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "outputFileTracingRoot")]
    pub output_file_tracing_root: Option<&'a str>,
    /// The files a route's trace leaves out, by its glob.
    #[cfg_attr(rust_js, rust_js::name = "outputFileTracingExcludes")]
    pub output_file_tracing_excludes: Option<&'a Dict<&'a [&'a str]>>,
    /// The files a route's trace takes in, by its glob.
    #[cfg_attr(rust_js, rust_js::name = "outputFileTracingIncludes")]
    pub output_file_tracing_includes: Option<&'a Dict<&'a [&'a str]>>,
    #[cfg_attr(rust_js, rust_js::name = "outputHashSalt")]
    pub output_hash_salt: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "watchOptions")]
    pub watch_options: Option<WatchOptions>,
    /// The bots streaming metadata waits for, by their user agents.
    #[cfg_attr(rust_js, rust_js::name = "htmlLimitedBots")]
    pub html_limited_bots: Option<&'a RegExp>,
    pub experimental: Option<ExperimentalConfig<'a>>,
    /// Behaviors Next.js deprecated, kept on.
    pub deprecated: Option<DeprecatedConfig>,
}

#[derive(Default)]
pub struct DeprecatedConfig {
    /// `Some(true)`: routes matched as before Next.js 16.4, loosely.
    #[cfg_attr(rust_js, rust_js::name = "looseRouteMatching")]
    pub loose_route_matching: Option<bool>,
}

/// A path's page, of a static export, as `ExportPathMap` has it.
pub struct ExportPathMapEntry<'a> {
    pub page: &'a str,
    pub query: Option<&'a Dict<QueryValue>>,
}

/// What `exportPathMap` is given besides the default map.
pub struct ExportPathMapContext {
    pub dev: bool,
    pub dir: String,
    #[cfg_attr(rust_js, rust_js::name = "outDir")]
    pub out_dir: Option<String>,
    #[cfg_attr(rust_js, rust_js::name = "distDir")]
    pub dist_dir: String,
    #[cfg_attr(rust_js, rust_js::name = "buildId")]
    pub build_id: String,
}

/// Internationalized routing's locales, as `I18NConfig` types it.
pub struct I18NConfig<'a> {
    #[cfg_attr(rust_js, rust_js::name = "defaultLocale")]
    pub default_locale: &'a str,
    /// Each domain's locales.
    pub domains: Option<&'a [DomainLocale<'a>]>,
    /// `Some(false)`: no locale detected of a request's `Accept-Language`.
    #[cfg_attr(rust_js, rust_js::name = "localeDetection")]
    pub locale_detection: Option<bool>,
    pub locales: &'a [&'a str],
}

/// A domain's locales, as `DomainLocale` types it.
pub struct DomainLocale<'a> {
    #[cfg_attr(rust_js, rust_js::name = "defaultLocale")]
    pub default_locale: &'a str,
    pub domain: &'a str,
    /// `Some(true)`: the domain is served over HTTP.
    pub http: Option<bool>,
    pub locales: Option<&'a [&'a str]>,
}

pub struct TypeScriptConfig<'a> {
    /// A build of TypeScript's errors goes on.
    #[cfg_attr(rust_js, rust_js::name = "ignoreBuildErrors")]
    pub ignore_build_errors: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "tsconfigPath")]
    pub tsconfig_path: Option<&'a str>,
}

/// A path's headers, as `Header` types it.
pub struct Header<'a> {
    /// The path, a pattern: `"/about"`, `"/blog/:slug"`.
    pub source: &'a str,
    /// `Some(false)`: `source` without the app's base path.
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<bool>,
    /// `Some(false)`: `source` without its locale.
    pub locale: Option<bool>,
    pub headers: &'a [HeaderEntry<'a>],
    /// What the request has, for the headers to be its.
    pub has: Option<&'a [RouteHas<'a>]>,
    /// What it hasn't.
    pub missing: Option<&'a [RouteHas<'a>]>,
}

/// A header's name and value.
pub struct HeaderEntry<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

/// A path served as another, as `Rewrite` types it.
pub struct Rewrite<'a> {
    pub source: &'a str,
    pub destination: &'a str,
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<bool>,
    pub locale: Option<bool>,
    pub has: Option<&'a [RouteHas<'a>]>,
    pub missing: Option<&'a [RouteHas<'a>]>,
}

/// What `rewrites` gives: rewrites after the app's files, or of each phase,
/// each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Rewrites<'a> {
    AfterFiles(Vec<Rewrite<'a>>),
    Phases(RewritePhases<'a>),
}

/// The rewrites before the app's files, after them, and after its dynamic
/// routes.
#[derive(Default)]
pub struct RewritePhases<'a> {
    #[cfg_attr(rust_js, rust_js::name = "beforeFiles")]
    pub before_files: Option<Vec<Rewrite<'a>>>,
    #[cfg_attr(rust_js, rust_js::name = "afterFiles")]
    pub after_files: Option<Vec<Rewrite<'a>>>,
    pub fallback: Option<Vec<Rewrite<'a>>>,
}

/// A path sent to another, as `Redirect` types it: for good or not, or by
/// its status, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Redirect<'a> {
    Permanent(PermanentRedirect<'a>),
    Status(StatusRedirect<'a>),
}

/// A redirect for good, a 308, or not, a 307.
pub struct PermanentRedirect<'a> {
    pub source: &'a str,
    pub destination: &'a str,
    pub permanent: bool,
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<bool>,
    pub locale: Option<bool>,
    pub has: Option<&'a [RouteHas<'a>]>,
    pub missing: Option<&'a [RouteHas<'a>]>,
    pub priority: Option<bool>,
}

/// A redirect of its status: `301`, `302`, `303`, `307` or `308`.
pub struct StatusRedirect<'a> {
    pub source: &'a str,
    pub destination: &'a str,
    #[cfg_attr(rust_js, rust_js::name = "statusCode")]
    pub status_code: u16,
    #[cfg_attr(rust_js, rust_js::name = "basePath")]
    pub base_path: Option<bool>,
    pub locale: Option<bool>,
    pub has: Option<&'a [RouteHas<'a>]>,
    pub missing: Option<&'a [RouteHas<'a>]>,
    pub priority: Option<bool>,
}

/// What a request has, as `RouteHas` types it: a header, a cookie or a
/// query's parameter, or its host, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum RouteHas<'a> {
    Keyed(KeyedRouteHas<'a>),
    Host(HostRouteHas<'a>),
}

/// A header, a cookie or a query's parameter of its name, and of its
/// value's pattern.
pub struct KeyedRouteHas<'a> {
    pub r#type: RouteHasType,
    pub key: &'a str,
    pub value: Option<&'a str>,
}

pub enum RouteHasType {
    #[cfg_attr(rust_js, rust_js::name = "header")]
    Header,
    #[cfg_attr(rust_js, rust_js::name = "cookie")]
    Cookie,
    #[cfg_attr(rust_js, rust_js::name = "query")]
    Query,
}

/// The host, of its pattern.
pub struct HostRouteHas<'a> {
    /// Always [`HostType::Host`].
    pub r#type: HostType,
    pub value: &'a str,
}

pub enum HostType {
    #[cfg_attr(rust_js, rust_js::name = "host")]
    Host,
}

/// What `webpack` is given besides its config, as `WebpackConfigContext`
/// types it.
pub struct WebpackConfigContext {
    pub dir: String,
    pub dev: bool,
    /// Whether it's the server's build.
    #[cfg_attr(rust_js, rust_js::name = "isServer")]
    pub is_server: bool,
    #[cfg_attr(rust_js, rust_js::name = "buildId")]
    pub build_id: String,
    /// The app's config, of Next.js's defaults.
    pub config: &'static Unknown,
    /// Next.js's loaders: `babel`.
    #[cfg_attr(rust_js, rust_js::name = "defaultLoaders")]
    pub default_loaders: &'static Unknown,
    #[cfg_attr(rust_js, rust_js::name = "totalPages")]
    pub total_pages: f64,
    /// webpack itself.
    pub webpack: &'static Unknown,
    /// The build's runtime, `None` of the client's.
    #[cfg_attr(rust_js, rust_js::name = "nextRuntime")]
    pub next_runtime: Option<NextRuntime>,
}

pub enum NextRuntime {
    #[cfg_attr(rust_js, rust_js::name = "nodejs")]
    Nodejs,
    #[cfg_attr(rust_js, rust_js::name = "edge")]
    Edge,
}

/// next/image's config, each `None` but what's given, as `ImageConfig`
/// types it.
#[derive(Default)]
pub struct ImageConfig<'a> {
    /// The widths of the devices an image is made for.
    #[cfg_attr(rust_js, rust_js::name = "deviceSizes")]
    pub device_sizes: Option<&'a [f64]>,
    /// The widths of an image of `sizes` smaller than a device.
    #[cfg_attr(rust_js, rust_js::name = "imageSizes")]
    pub image_sizes: Option<&'a [f64]>,
    pub loader: Option<LoaderValue>,
    /// The URL images are optimized at: `"/_next/image"`.
    pub path: Option<&'a str>,
    /// The module of its loader, a `custom` one.
    #[cfg_attr(rust_js, rust_js::name = "loaderFile")]
    pub loader_file: Option<&'a str>,
    /// The hosts remote images may be from, what `remotePatterns` replaced.
    pub domains: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "disableStaticImages")]
    pub disable_static_images: Option<bool>,
    /// How long an optimized image is cached, in seconds.
    #[cfg_attr(rust_js, rust_js::name = "minimumCacheTTL")]
    pub minimum_cache_ttl: Option<f64>,
    pub formats: Option<&'a [ImageFormat]>,
    #[cfg_attr(rust_js, rust_js::name = "maximumDiskCacheSize")]
    pub maximum_disk_cache_size: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maximumRedirects")]
    pub maximum_redirects: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maximumResponseBody")]
    pub maximum_response_body: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "dangerouslyAllowLocalIP")]
    pub dangerously_allow_local_ip: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "dangerouslyAllowSVG")]
    pub dangerously_allow_svg: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "contentSecurityPolicy")]
    pub content_security_policy: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "contentDispositionType")]
    pub content_disposition_type: Option<ContentDispositionType>,
    /// The URLs remote images may be from.
    #[cfg_attr(rust_js, rust_js::name = "remotePatterns")]
    pub remote_patterns: Option<&'a [RemotePatternOrUrl<'a>]>,
    /// The paths local images may be at.
    #[cfg_attr(rust_js, rust_js::name = "localPatterns")]
    pub local_patterns: Option<&'a [LocalPattern<'a>]>,
    /// The qualities an image may be of.
    pub qualities: Option<&'a [f64]>,
    /// Each image as it is, not optimized.
    pub unoptimized: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "customCacheHandler")]
    pub custom_cache_handler: Option<bool>,
}

/// next/image's loader, as `LoaderValue` types it.
pub enum LoaderValue {
    #[cfg_attr(rust_js, rust_js::name = "default")]
    Default,
    #[cfg_attr(rust_js, rust_js::name = "imgix")]
    Imgix,
    #[cfg_attr(rust_js, rust_js::name = "cloudinary")]
    Cloudinary,
    #[cfg_attr(rust_js, rust_js::name = "akamai")]
    Akamai,
    #[cfg_attr(rust_js, rust_js::name = "custom")]
    Custom,
}

pub enum ImageFormat {
    #[cfg_attr(rust_js, rust_js::name = "image/avif")]
    Avif,
    #[cfg_attr(rust_js, rust_js::name = "image/webp")]
    Webp,
}

pub enum ContentDispositionType {
    #[cfg_attr(rust_js, rust_js::name = "inline")]
    Inline,
    #[cfg_attr(rust_js, rust_js::name = "attachment")]
    Attachment,
}

/// Where a remote image may be from, `URL | RemotePattern`: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum RemotePatternOrUrl<'a> {
    Url(&'a URL),
    Pattern(RemotePattern<'a>),
}

/// The URLs a remote image may be at, as `RemotePattern` types them.
pub struct RemotePattern<'a> {
    pub protocol: Option<Protocol>,
    pub hostname: &'a str,
    pub port: Option<&'a str>,
    pub pathname: Option<&'a str>,
    pub search: Option<&'a str>,
}

pub enum Protocol {
    #[cfg_attr(rust_js, rust_js::name = "http")]
    Http,
    #[cfg_attr(rust_js, rust_js::name = "https")]
    Https,
}

/// The paths a local image may be at, as `LocalPattern` types them.
#[derive(Default)]
pub struct LocalPattern<'a> {
    pub pathname: Option<&'a str>,
    pub search: Option<&'a str>,
}

/// The development server's indicator, `false | { position }`: each the
/// value itself, `false` none.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum DevIndicators {
    Bool(bool),
    Options(DevIndicatorsOptions),
}

pub struct DevIndicatorsOptions {
    pub position: Option<DevIndicatorsPosition>,
}

pub enum DevIndicatorsPosition {
    #[cfg_attr(rust_js, rust_js::name = "top-left")]
    TopLeft,
    #[cfg_attr(rust_js, rust_js::name = "top-right")]
    TopRight,
    #[cfg_attr(rust_js, rust_js::name = "bottom-left")]
    BottomLeft,
    #[cfg_attr(rust_js, rust_js::name = "bottom-right")]
    BottomRight,
}

#[derive(Default)]
pub struct OnDemandEntries {
    /// How long a page is kept, in milliseconds.
    #[cfg_attr(rust_js, rust_js::name = "maxInactiveAge")]
    pub max_inactive_age: Option<f64>,
    /// How many pages are kept.
    #[cfg_attr(rust_js, rust_js::name = "pagesBufferLength")]
    pub pages_buffer_length: Option<f64>,
}

/// The React Compiler, `boolean | ReactCompilerOptions`: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ReactCompiler {
    Bool(bool),
    Options(ReactCompilerOptions),
}

#[derive(Default)]
pub struct ReactCompilerOptions {
    #[cfg_attr(rust_js, rust_js::name = "compilationMode")]
    pub compilation_mode: Option<CompilationMode>,
    #[cfg_attr(rust_js, rust_js::name = "panicThreshold")]
    pub panic_threshold: Option<PanicThreshold>,
    pub environment: Option<ReactCompilerEnvironment>,
}

#[derive(Default)]
pub struct ReactCompilerEnvironment {
    #[cfg_attr(rust_js, rust_js::name = "enablePreserveExistingMemoizationGuarantees")]
    pub enable_preserve_existing_memoization_guarantees: Option<bool>,
}

/// Which components and hooks the React Compiler compiles.
pub enum CompilationMode {
    #[cfg_attr(rust_js, rust_js::name = "infer")]
    Infer,
    /// Those of `"use memo"`.
    #[cfg_attr(rust_js, rust_js::name = "annotation")]
    Annotation,
    #[cfg_attr(rust_js, rust_js::name = "all")]
    All,
}

/// What errors of the React Compiler's fail the build.
pub enum PanicThreshold {
    #[cfg_attr(rust_js, rust_js::name = "none")]
    None,
    #[cfg_attr(rust_js, rust_js::name = "critical_errors")]
    CriticalErrors,
    #[cfg_attr(rust_js, rust_js::name = "all_errors")]
    AllErrors,
}

#[derive(Default)]
pub struct HttpAgentOptions {
    #[cfg_attr(rust_js, rust_js::name = "keepAlive")]
    pub keep_alive: Option<bool>,
}

pub enum CrossOrigin {
    #[cfg_attr(rust_js, rust_js::name = "anonymous")]
    Anonymous,
    #[cfg_attr(rust_js, rust_js::name = "use-credentials")]
    UseCredentials,
}

/// The compiler's transforms, as `NextConfig`'s `compiler` types them.
#[derive(Default)]
pub struct CompilerConfig<'a> {
    /// JSX's props of `data-test`, or of these patterns, removed.
    #[cfg_attr(rust_js, rust_js::name = "reactRemoveProperties")]
    pub react_remove_properties: Option<ReactRemoveProperties<'a>>,
    pub relay: Option<RelayConfig<'a>>,
    /// `console.*` calls removed.
    #[cfg_attr(rust_js, rust_js::name = "removeConsole")]
    pub remove_console: Option<RemoveConsole<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "styledComponents")]
    pub styled_components: Option<StyledComponents<'a>>,
    pub emotion: Option<Emotion<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "styledJsx")]
    pub styled_jsx: Option<StyledJsx>,
    /// Each name, replaced with its value, in the app's code.
    pub define: Option<&'a Dict<DefineValue<'a>>>,
    /// Each name, replaced with its value, in the server's code.
    #[cfg_attr(rust_js, rust_js::name = "defineServer")]
    pub define_server: Option<&'a Dict<DefineValue<'a>>>,
    /// What runs after a production build's compile.
    #[cfg_attr(rust_js, rust_js::name = "runAfterProductionCompile")]
    pub run_after_production_compile: Option<Box<dyn Fn(CompileMetadata) -> Promise<()>>>,
}

/// `boolean | { properties }`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ReactRemoveProperties<'a> {
    Bool(bool),
    Options(ReactRemovePropertiesOptions<'a>),
}

/// The patterns of the props removed.
pub struct ReactRemovePropertiesOptions<'a> {
    pub properties: Option<&'a [&'a str]>,
}

/// Relay's config.
pub struct RelayConfig<'a> {
    pub src: &'a str,
    #[cfg_attr(rust_js, rust_js::name = "artifactDirectory")]
    pub artifact_directory: Option<&'a str>,
    pub language: Option<RelayLanguage>,
    #[cfg_attr(rust_js, rust_js::name = "eagerEsModules")]
    pub eager_es_modules: Option<bool>,
}

pub enum RelayLanguage {
    #[cfg_attr(rust_js, rust_js::name = "typescript")]
    Typescript,
    #[cfg_attr(rust_js, rust_js::name = "javascript")]
    Javascript,
    #[cfg_attr(rust_js, rust_js::name = "flow")]
    Flow,
}

/// `boolean | { exclude }`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum RemoveConsole<'a> {
    Bool(bool),
    Options(RemoveConsoleOptions<'a>),
}

/// The `console` methods kept: `["error"]`.
pub struct RemoveConsoleOptions<'a> {
    pub exclude: Option<&'a [&'a str]>,
}

/// `boolean | StyledComponentsConfig`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StyledComponents<'a> {
    Bool(bool),
    Config(StyledComponentsConfig<'a>),
}

#[derive(Default)]
pub struct StyledComponentsConfig<'a> {
    #[cfg_attr(rust_js, rust_js::name = "displayName")]
    pub display_name: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "topLevelImportPaths")]
    pub top_level_import_paths: Option<&'a [&'a str]>,
    pub ssr: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "fileName")]
    pub file_name: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "meaninglessFileNames")]
    pub meaningless_file_names: Option<&'a [&'a str]>,
    pub minify: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "transpileTemplateLiterals")]
    pub transpile_template_literals: Option<bool>,
    pub namespace: Option<&'a str>,
    pub pure: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "cssProp")]
    pub css_prop: Option<bool>,
}

/// `boolean | EmotionConfig`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Emotion<'a> {
    Bool(bool),
    Config(EmotionConfig<'a>),
}

#[derive(Default)]
pub struct EmotionConfig<'a> {
    #[cfg_attr(rust_js, rust_js::name = "sourceMap")]
    pub source_map: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "autoLabel")]
    pub auto_label: Option<AutoLabel>,
    /// A label's format: `"[local]"`.
    #[cfg_attr(rust_js, rust_js::name = "labelFormat")]
    pub label_format: Option<&'a str>,
    /// Each import's of each module, as emotion's own.
    #[cfg_attr(rust_js, rust_js::name = "importMap")]
    pub import_map: Option<&'a Dict<&'a Dict<EmotionImport<'a>>>>,
}

pub enum AutoLabel {
    #[cfg_attr(rust_js, rust_js::name = "dev-only")]
    DevOnly,
    #[cfg_attr(rust_js, rust_js::name = "always")]
    Always,
    #[cfg_attr(rust_js, rust_js::name = "never")]
    Never,
}

/// What an import is of emotion's: its module and export.
#[derive(Default)]
pub struct EmotionImport<'a> {
    #[cfg_attr(rust_js, rust_js::name = "canonicalImport")]
    pub canonical_import: Option<(&'a str, &'a str)>,
    #[cfg_attr(rust_js, rust_js::name = "styledBaseImport")]
    pub styled_base_import: Option<(&'a str, &'a str)>,
}

/// `boolean | { useLightningcss }`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StyledJsx {
    Bool(bool),
    Options(StyledJsxOptions),
}

pub struct StyledJsxOptions {
    #[cfg_attr(rust_js, rust_js::name = "useLightningcss")]
    pub use_lightningcss: Option<bool>,
}

/// A defined name's value, `string | number | boolean`: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum DefineValue<'a> {
    Str(&'a str),
    Number(f64),
    Bool(bool),
}

/// What `runAfterProductionCompile` is given.
pub struct CompileMetadata {
    #[cfg_attr(rust_js, rust_js::name = "projectDir")]
    pub project_dir: String,
    #[cfg_attr(rust_js, rust_js::name = "distDir")]
    pub dist_dir: String,
}

/// A standalone server, or a static export.
pub enum Output {
    #[cfg_attr(rust_js, rust_js::name = "standalone")]
    Standalone,
    #[cfg_attr(rust_js, rust_js::name = "export")]
    Export,
}

/// Turbopack's options, as `TurbopackOptions` types them.
#[derive(Default)]
pub struct TurbopackOptions<'a> {
    /// The module each import is of instead.
    #[cfg_attr(rust_js, rust_js::name = "resolveAlias")]
    pub resolve_alias: Option<&'a Dict<ResolveAlias<'a>>>,
    /// The extensions an import may leave out.
    #[cfg_attr(rust_js, rust_js::name = "resolveExtensions")]
    pub resolve_extensions: Option<&'a [&'a str]>,
    /// The loaders of each file, by its glob.
    pub rules: Option<&'a Dict<TurbopackRuleConfigCollection<'a>>>,
    /// The directory it resolves files below.
    pub root: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "debugIds")]
    pub debug_ids: Option<bool>,
    /// The issues it doesn't show.
    #[cfg_attr(rust_js, rust_js::name = "ignoreIssue")]
    pub ignore_issue: Option<&'a [IgnoreIssue<'a>]>,
    #[cfg_attr(rust_js, rust_js::name = "chunkLoadingGlobal")]
    pub chunk_loading_global: Option<&'a str>,
}

/// What an import is aliased to, `false | string | string[] |
/// Record<string, false | string | string[]>`: nothing, a module, the
/// first of these that resolves, or one of each condition; each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ResolveAlias<'a> {
    /// `false`: an import of nothing, an empty module.
    Bool(bool),
    Module(&'a str),
    Modules(&'a [&'a str]),
    Conditional(&'a Dict<ResolveAliasTarget<'a>>),
}

/// A condition's module, nothing, or the first of these that resolves.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ResolveAliasTarget<'a> {
    Bool(bool),
    Module(&'a str),
    Modules(&'a [&'a str]),
}

/// A glob's rule, or its loaders and rules, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackRuleConfigCollection<'a> {
    Rule(TurbopackRuleConfigItem<'a>),
    Many(&'a [TurbopackLoaderOrRule<'a>]),
}

/// A loader, by its module or of its options, or a rule: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackLoaderOrRule<'a> {
    Module(&'a str),
    Options(TurbopackLoaderOptions<'a>),
    Rule(TurbopackRuleConfigItem<'a>),
}

/// A loader, by its module, or of its options: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackLoaderItem<'a> {
    Module(&'a str),
    Options(TurbopackLoaderOptions<'a>),
}

pub struct TurbopackLoaderOptions<'a> {
    pub loader: &'a str,
    pub options: Option<&'a Dict<Json<'a>>>,
}

/// A rule: its loaders, what its files are as, of what condition.
#[derive(Default)]
pub struct TurbopackRuleConfigItem<'a> {
    pub loaders: Option<&'a [TurbopackLoaderItem<'a>]>,
    /// The file's name the loaders' output has: `"*.js"`.
    pub r#as: Option<&'a str>,
    pub condition: Option<TurbopackRuleCondition<'a>>,
    pub r#type: Option<TurbopackModuleType>,
}

/// What a file is for a rule to be its, as `TurbopackRuleCondition` types
/// it: each the value itself, a built-in condition its name.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackRuleCondition<'a> {
    All(TurbopackRuleAll<'a>),
    Any(TurbopackRuleAny<'a>),
    Not(TurbopackRuleNot<'a>),
    #[cfg_attr(rust_js, rust_js::name = "browser")]
    Browser,
    /// A file of node_modules'.
    #[cfg_attr(rust_js, rust_js::name = "foreign")]
    Foreign,
    #[cfg_attr(rust_js, rust_js::name = "development")]
    Development,
    #[cfg_attr(rust_js, rust_js::name = "production")]
    Production,
    #[cfg_attr(rust_js, rust_js::name = "node")]
    Node,
    #[cfg_attr(rust_js, rust_js::name = "edge-light")]
    EdgeLight,
    Match(TurbopackRuleMatch<'a>),
}

/// Each of these conditions.
pub struct TurbopackRuleAll<'a> {
    pub all: &'a [TurbopackRuleCondition<'a>],
}

/// Any of these conditions.
pub struct TurbopackRuleAny<'a> {
    pub any: &'a [TurbopackRuleCondition<'a>],
}

/// Not this condition.
pub struct TurbopackRuleNot<'a> {
    pub not: &'a TurbopackRuleCondition<'a>,
}

/// A file of this path, content, query or content type.
#[derive(Default)]
pub struct TurbopackRuleMatch<'a> {
    pub path: Option<StrOrRegExp<'a>>,
    pub content: Option<&'a RegExp>,
    /// Its import's query: `"?raw"`.
    pub query: Option<StrOrRegExp<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "contentType")]
    pub content_type: Option<StrOrRegExp<'a>>,
}

/// `string | RegExp`: a glob, or a pattern, each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StrOrRegExp<'a> {
    Str(&'a str),
    RegExp(&'a RegExp),
}

/// What a file is as, as `TurbopackModuleType` types it.
pub enum TurbopackModuleType {
    #[cfg_attr(rust_js, rust_js::name = "asset")]
    Asset,
    #[cfg_attr(rust_js, rust_js::name = "ecmascript")]
    Ecmascript,
    #[cfg_attr(rust_js, rust_js::name = "typescript")]
    Typescript,
    #[cfg_attr(rust_js, rust_js::name = "css")]
    Css,
    #[cfg_attr(rust_js, rust_js::name = "css-module")]
    CssModule,
    #[cfg_attr(rust_js, rust_js::name = "json")]
    Json,
    #[cfg_attr(rust_js, rust_js::name = "wasm")]
    Wasm,
    #[cfg_attr(rust_js, rust_js::name = "raw")]
    Raw,
    #[cfg_attr(rust_js, rust_js::name = "node")]
    Node,
    #[cfg_attr(rust_js, rust_js::name = "bytes")]
    Bytes,
    #[cfg_attr(rust_js, rust_js::name = "text")]
    Text,
}

/// An issue Turbopack doesn't show, of its path, title and description.
pub struct IgnoreIssue<'a> {
    pub path: StrOrRegExp<'a>,
    pub title: Option<StrOrRegExp<'a>>,
    pub description: Option<StrOrRegExp<'a>>,
}

/// A package's imports, as imports of its modules.
pub struct ModularizeImport<'a> {
    /// The module of each import: `"lodash/{{member}}"`, or by its name.
    pub transform: ModularizeTransform<'a>,
    #[cfg_attr(rust_js, rust_js::name = "preventFullImport")]
    pub prevent_full_import: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "skipDefaultConversion")]
    pub skip_default_conversion: Option<bool>,
}

/// `string | Record<string, string>`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ModularizeTransform<'a> {
    Str(&'a str),
    Each(&'a Dict<&'a str>),
}

/// What a development server logs, `LoggingConfig | false`: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Logging<'a> {
    Bool(bool),
    Config(LoggingConfig<'a>),
}

#[derive(Default)]
pub struct LoggingConfig<'a> {
    pub fetches: Option<FetchesLogging>,
    #[cfg_attr(rust_js, rust_js::name = "incomingRequests")]
    pub incoming_requests: Option<IncomingRequests<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "serverFunctions")]
    pub server_functions: Option<bool>,
    /// The browser's logs, in the terminal: each, or of these levels.
    #[cfg_attr(rust_js, rust_js::name = "browserToTerminal")]
    pub browser_to_terminal: Option<BrowserToTerminal>,
}

#[derive(Default)]
pub struct FetchesLogging {
    #[cfg_attr(rust_js, rust_js::name = "fullUrl")]
    pub full_url: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "hmrRefreshes")]
    pub hmr_refreshes: Option<bool>,
}

/// `boolean | IncomingRequestLoggingConfig`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum IncomingRequests<'a> {
    Bool(bool),
    Config(IncomingRequestLoggingConfig<'a>),
}

/// The requests not logged, by their URLs' patterns.
pub struct IncomingRequestLoggingConfig<'a> {
    pub ignore: Option<&'a [&'a RegExp]>,
}

/// `boolean | "error" | "warn"`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum BrowserToTerminal {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "error")]
    Error,
    #[cfg_attr(rust_js, rust_js::name = "warn")]
    Warn,
}

/// A `cacheLife` profile's times, in seconds.
#[derive(Default)]
pub struct CacheLifeProfile {
    pub stale: Option<f64>,
    pub revalidate: Option<f64>,
    pub expire: Option<f64>,
}

#[derive(Default)]
pub struct WatchOptions {
    /// How often files are polled, in milliseconds.
    #[cfg_attr(rust_js, rust_js::name = "pollIntervalMs")]
    pub poll_interval_ms: Option<f64>,
}

/// Next.js's experimental options, each `None` but what's given, as
/// `ExperimentalConfig` types them: what may change, or go, in any release.
#[derive(Default)]
pub struct ExperimentalConfig<'a> {
    /// The coding agents' rules Next.js upgrades to, `false` none.
    #[cfg_attr(rust_js, rust_js::name = "agentUpgrade")]
    pub agent_upgrade: Option<AgentUpgrade>,
    #[cfg_attr(rust_js, rust_js::name = "agentFeedback")]
    pub agent_feedback: Option<bool>,
    /// Directories outside its root Turbopack reads, by their names.
    #[cfg_attr(rust_js, rust_js::name = "turbopackAdditionalRoots")]
    pub turbopack_additional_roots: Option<&'a Dict<TurbopackAdditionalRoot<'a>>>,
    #[cfg_attr(rust_js, rust_js::name = "outputHashSalt")]
    pub output_hash_salt: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "coldCacheBadge")]
    pub cold_cache_badge: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "collapseAdapterRoutes")]
    pub collapse_adapter_routes: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useSkewCookie")]
    pub use_skew_cookie: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "cacheHandlers")]
    pub cache_handlers: Option<&'a Dict<Option<&'a str>>>,
    #[cfg_attr(rust_js, rust_js::name = "multiZoneDraftMode")]
    pub multi_zone_draft_mode: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "appNavFailHandling")]
    pub app_nav_fail_handling: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "parallelRouteMetadata")]
    pub parallel_route_metadata: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "prerenderEarlyExit")]
    pub prerender_early_exit: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "linkNoTouchStart")]
    pub link_no_touch_start: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "caseSensitiveRoutes")]
    pub case_sensitive_routes: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "clientParamParsingOrigins")]
    pub client_param_parsing_origins: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "cachedNavigations")]
    pub cached_navigations: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "dynamicOnHover")]
    pub dynamic_on_hover: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "reactBrowserBailout")]
    pub react_browser_bailout: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useOffline")]
    pub use_offline: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "optimisticRouting")]
    pub optimistic_routing: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "concurrentRouterQueue")]
    pub concurrent_router_queue: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "instrumentationClientRouterTransitionEvents")]
    pub instrumentation_client_router_transition_events: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "varyParams")]
    pub vary_params: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "prefetchInlining")]
    pub prefetch_inlining: Option<PrefetchInlining>,
    #[cfg_attr(rust_js, rust_js::name = "preloadEntriesOnStart")]
    pub preload_entries_on_start: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "clientRouterFilter")]
    pub client_router_filter: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "clientRouterFilterRedirects")]
    pub client_router_filter_redirects: Option<bool>,
    /// How long the client's router cache keeps a page, in seconds.
    #[cfg_attr(rust_js, rust_js::name = "staleTimes")]
    pub stale_times: Option<StaleTimes>,
    #[cfg_attr(rust_js, rust_js::name = "cacheLife")]
    pub cache_life: Option<&'a Dict<CacheLifeProfile>>,
    #[cfg_attr(rust_js, rust_js::name = "clientRouterFilterAllowedRate")]
    pub client_router_filter_allowed_rate: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "externalMiddlewareRewritesResolve")]
    pub external_middleware_rewrites_resolve: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "externalProxyRewritesResolve")]
    pub external_proxy_rewrites_resolve: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "exposeTestingApiInProductionBuild")]
    pub expose_testing_api_in_production_build: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "requestInsights")]
    pub request_insights: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "extensionAlias")]
    pub extension_alias: Option<&'a Dict<Json<'a>>>,
    #[cfg_attr(rust_js, rust_js::name = "allowedRevalidateHeaderKeys")]
    pub allowed_revalidate_header_keys: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "fetchCacheKeyPrefix")]
    pub fetch_cache_key_prefix: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "imgOptConcurrency")]
    pub img_opt_concurrency: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "imgOptOperationCache")]
    pub img_opt_operation_cache: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "imgOptTimeoutInSeconds")]
    pub img_opt_timeout_in_seconds: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "imgOptMaxInputPixels")]
    pub img_opt_max_input_pixels: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "imgOptSequentialRead")]
    pub img_opt_sequential_read: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "imgOptMozjpeg")]
    pub img_opt_mozjpeg: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "optimisticClientCache")]
    pub optimistic_client_cache: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "expireTime")]
    pub expire_time: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "middlewarePrefetch")]
    pub middleware_prefetch: Option<ProxyPrefetch>,
    #[cfg_attr(rust_js, rust_js::name = "proxyPrefetch")]
    pub proxy_prefetch: Option<ProxyPrefetch>,
    #[cfg_attr(rust_js, rust_js::name = "manualClientBasePath")]
    pub manual_client_base_path: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "cssChunking")]
    pub css_chunking: Option<CssChunkingConfig>,
    #[cfg_attr(rust_js, rust_js::name = "devMemoryThresholdRestart")]
    pub dev_memory_threshold_restart: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "disablePostcssPresetEnv")]
    pub disable_postcss_preset_env: Option<bool>,
    pub cpus: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "memoryBasedWorkersCount")]
    pub memory_based_workers_count: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "proxyTimeout")]
    pub proxy_timeout: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "isrFlushToDisk")]
    pub isr_flush_to_disk: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "workerThreads")]
    pub worker_threads: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "optimizeCss")]
    pub optimize_css: Option<OptimizeCss<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "nextScriptWorkers")]
    pub next_script_workers: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "scrollRestoration")]
    pub scroll_restoration: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "externalDir")]
    pub external_dir: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "disableOptimizedLoading")]
    pub disable_optimized_loading: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "gzipSize")]
    pub gzip_size: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "craCompat")]
    pub cra_compat: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "esmExternals")]
    pub esm_externals: Option<EsmExternals>,
    #[cfg_attr(rust_js, rust_js::name = "fullySpecified")]
    pub fully_specified: Option<bool>,
    /// webpack's `buildHttp`, which Next.js types as `any`.
    #[cfg_attr(rust_js, rust_js::name = "urlImports")]
    pub url_imports: Option<Json<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "swcTraceProfiling")]
    pub swc_trace_profiling: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "forceSwcTransforms")]
    pub force_swc_transforms: Option<bool>,
    /// Each SWC plugin's module, and its options.
    #[cfg_attr(rust_js, rust_js::name = "swcPlugins")]
    pub swc_plugins: Option<&'a [(&'a str, &'a Dict<Json<'a>>)]>,
    #[cfg_attr(rust_js, rust_js::name = "swcEnvOptions")]
    pub swc_env_options: Option<SwcEnvOptions<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "largePageDataBytes")]
    pub large_page_data_bytes: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "fallbackNodePolyfills")]
    pub fallback_node_polyfills: Option<bool>,
    pub sri: Option<Sri>,
    #[cfg_attr(rust_js, rust_js::name = "webVitalsAttribution")]
    pub web_vitals_attribution: Option<&'a [WebVital]>,
    #[cfg_attr(rust_js, rust_js::name = "optimizePackageImports")]
    pub optimize_package_imports: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "optimizeServerReact")]
    pub optimize_server_react: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "strictRouteTypes")]
    pub strict_route_types: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useTypeScriptCli")]
    pub use_type_script_cli: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "transitionIndicator")]
    pub transition_indicator: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "gestureTransition")]
    pub gesture_transition: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackMemoryEviction")]
    pub turbopack_memory_eviction: Option<TurbopackMemoryEviction>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackGc")]
    pub turbopack_gc: Option<TurbopackGc>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackPluginRuntimeStrategy")]
    pub turbopack_plugin_runtime_strategy: Option<TurbopackPluginRuntimeStrategy>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackMinify")]
    pub turbopack_minify: Option<TurbopackMinify>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackImportTypeBytes")]
    pub turbopack_import_type_bytes: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackScopeHoisting")]
    pub turbopack_scope_hoisting: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackSharedRuntime")]
    pub turbopack_shared_runtime: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackChunking")]
    pub turbopack_chunking: Option<TurbopackChunking<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackWorkerAssetPrefix")]
    pub turbopack_worker_asset_prefix: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackClientSideNestedAsyncChunking")]
    pub turbopack_client_side_nested_async_chunking: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackServerSideNestedAsyncChunking")]
    pub turbopack_server_side_nested_async_chunking: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackLazyDynamicImports")]
    pub turbopack_lazy_dynamic_imports: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackLazyDynamicImportsSSR")]
    pub turbopack_lazy_dynamic_imports_ssr: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackFileSystemCacheForDev")]
    pub turbopack_file_system_cache_for_dev: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackFileSystemCacheForBuild")]
    pub turbopack_file_system_cache_for_build: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackSeedCacheFromWorktree")]
    pub turbopack_seed_cache_from_worktree: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackStaleOutputMaxAge")]
    pub turbopack_stale_output_max_age: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackSourceMaps")]
    pub turbopack_source_maps: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackInputSourceMaps")]
    pub turbopack_input_source_maps: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackModuleFragments")]
    pub turbopack_module_fragments: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackRemoveUnusedImports")]
    pub turbopack_remove_unused_imports: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackRemoveUnusedExports")]
    pub turbopack_remove_unused_exports: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackInferModuleSideEffects")]
    pub turbopack_infer_module_side_effects: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackCjsTreeShaking")]
    pub turbopack_cjs_tree_shaking: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackMangleExportNames")]
    pub turbopack_mangle_export_names: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackMangleViaMaterializedNamespaceObject")]
    pub turbopack_mangle_via_materialized_namespace_object: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackCjsScopeHoisting")]
    pub turbopack_cjs_scope_hoisting: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackCrossModuleConstants")]
    pub turbopack_cross_module_constants: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackUseBuiltinBabel")]
    pub turbopack_use_builtin_babel: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackUseBuiltinSass")]
    pub turbopack_use_builtin_sass: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackLocalPostcssConfig")]
    pub turbopack_local_postcss_config: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackModuleIds")]
    pub turbopack_module_ids: Option<TurbopackModuleIds>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackServerFastRefresh")]
    pub turbopack_server_fast_refresh: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "mdxRs")]
    pub mdx_rs: Option<MdxRs<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "typedRoutes")]
    pub typed_routes: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "typedEnv")]
    pub typed_env: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "parallelServerCompiles")]
    pub parallel_server_compiles: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "parallelServerBuildTraces")]
    pub parallel_server_build_traces: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "webpackBuildWorker")]
    pub webpack_build_worker: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "webpackMemoryOptimizations")]
    pub webpack_memory_optimizations: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "clientTraceMetadata")]
    pub client_trace_metadata: Option<&'a [&'a str]>,
    /// Partial Prerendering, of each route, or of those that opt in.
    pub ppr: Option<ExperimentalPPRConfig>,
    pub taint: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "blockingSSR")]
    pub blocking_ssr: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "removeUncaughtErrorAndRejectionListeners")]
    pub remove_uncaught_error_and_rejection_listeners: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "validateRSCRequestHeaders")]
    pub validate_rsc_request_headers: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "serverActions")]
    pub server_actions: Option<ServerActions<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "maxPostponedStateSize")]
    pub max_postponed_state_size: Option<SizeLimit<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "disableResumeDataCacheCompression")]
    pub disable_resume_data_cache_compression: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "serverMinification")]
    pub server_minification: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "serverSourceMaps")]
    pub server_source_maps: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useWasmBinary")]
    pub use_wasm_binary: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useLightningcss")]
    pub use_lightningcss: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "lightningCssFeatures")]
    pub lightning_css_features: Option<LightningCssFeatures<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "testProxy")]
    pub test_proxy: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "defaultTestRunner")]
    pub default_test_runner: Option<SupportedTestRunners>,
    #[cfg_attr(rust_js, rust_js::name = "allowDevelopmentBuild")]
    pub allow_development_build: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "bundlePagesExternals")]
    pub bundle_pages_externals: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "serverComponentsExternalPackages")]
    pub server_components_external_packages: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "reactDebugChannel")]
    pub react_debug_channel: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "cacheComponents")]
    pub cache_components: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "instantInsights")]
    pub instant_insights: Option<InstantInsights>,
    #[cfg_attr(rust_js, rust_js::name = "devValidationWorker")]
    pub dev_validation_worker: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "staticGenerationRetryCount")]
    pub static_generation_retry_count: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "staticGenerationMaxConcurrency")]
    pub static_generation_max_concurrency: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "staticGenerationMinPagesPerWorker")]
    pub static_generation_min_pages_per_worker: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "serverComponentsHmrCache")]
    pub server_components_hmr_cache: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "serverComponentsHmrCancellation")]
    pub server_components_hmr_cancellation: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "inlineCss")]
    pub inline_css: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "authInterrupts")]
    pub auth_interrupts: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useCacheTimeout")]
    pub use_cache_timeout: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "useCache")]
    pub use_cache: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "durableUseCacheEntries")]
    pub durable_use_cache_entries: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "useCacheStaticRootParamTracking")]
    pub use_cache_static_root_param_tracking: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "slowModuleDetection")]
    pub slow_module_detection: Option<SlowModuleDetection>,
    #[cfg_attr(rust_js, rust_js::name = "globalNotFound")]
    pub global_not_found: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "explicitParallelRouteChildren")]
    pub explicit_parallel_route_children: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "turbopackRustReactCompiler")]
    pub turbopack_rust_react_compiler: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "browserDebugInfoInTerminal")]
    pub browser_debug_info_in_terminal: Option<BrowserDebugInfoInTerminal>,
    #[cfg_attr(rust_js, rust_js::name = "middlewareClientMaxBodySize")]
    pub middleware_client_max_body_size: Option<SizeLimit<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "proxyClientMaxBodySize")]
    pub proxy_client_max_body_size: Option<SizeLimit<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "mcpServer")]
    pub mcp_server: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "exposeRuntimeErrorsToHMR")]
    pub expose_runtime_errors_to_hmr: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "lockDistDir")]
    pub lock_dist_dir: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "hideLogsAfterAbort")]
    pub hide_logs_after_abort: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "runtimeServerDeploymentId")]
    pub runtime_server_deployment_id: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "supportsImmutableAssets")]
    pub supports_immutable_assets: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "deferredEntries")]
    pub deferred_entries: Option<&'a [&'a str]>,
    /// What runs before the deferred entries are built.
    #[cfg_attr(rust_js, rust_js::name = "onBeforeDeferredEntries")]
    pub on_before_deferred_entries: Option<Box<dyn Fn() -> Promise<()>>>,
    #[cfg_attr(rust_js, rust_js::name = "reportSystemEnvInlining")]
    pub report_system_env_inlining: Option<ReportLevel>,
}

/// `boolean | { maxSize, maxBundleSize }`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum PrefetchInlining {
    Bool(bool),
    Options(PrefetchInliningOptions),
}

#[derive(Default)]
pub struct PrefetchInliningOptions {
    #[cfg_attr(rust_js, rust_js::name = "maxSize")]
    pub max_size: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maxBundleSize")]
    pub max_bundle_size: Option<f64>,
}

#[derive(Default)]
pub struct StaleTimes {
    pub dynamic: Option<f64>,
    pub r#static: Option<f64>,
}

/// How a proxy's prefetch is: strict, or flexible.
pub enum ProxyPrefetch {
    #[cfg_attr(rust_js, rust_js::name = "strict")]
    Strict,
    #[cfg_attr(rust_js, rust_js::name = "flexible")]
    Flexible,
}

/// How CSS is chunked, as `CssChunkingConfig` types it: on or off, by its
/// mode's name, or of its options; each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum CssChunkingConfig {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "strict")]
    Strict,
    #[cfg_attr(rust_js, rust_js::name = "loose")]
    Loose,
    #[cfg_attr(rust_js, rust_js::name = "graph")]
    Graph,
    Typed(CssChunkingTyped),
    GraphOptions(CssChunkingGraph),
}

/// `{ type: "strict" }` or `{ type: "loose" }`.
pub struct CssChunkingTyped {
    pub r#type: CssChunkingType,
}

pub enum CssChunkingType {
    #[cfg_attr(rust_js, rust_js::name = "strict")]
    Strict,
    #[cfg_attr(rust_js, rust_js::name = "loose")]
    Loose,
}

/// `{ type: "graph", requestCost, weightDistribution }`.
pub struct CssChunkingGraph {
    /// Always [`GraphType::Graph`].
    pub r#type: GraphType,
    #[cfg_attr(rust_js, rust_js::name = "requestCost")]
    pub request_cost: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "weightDistribution")]
    pub weight_distribution: Option<f64>,
}

pub enum GraphType {
    #[cfg_attr(rust_js, rust_js::name = "graph")]
    Graph,
}

/// `boolean | Record<string, unknown>`: on, or Critters' options; each the
/// value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OptimizeCss<'a> {
    Bool(bool),
    Options(&'a Dict<Json<'a>>),
}

/// `boolean | "loose"`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum EsmExternals {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "loose")]
    Loose,
}

/// SWC's `env` options, its polyfills of core-js.
#[derive(Default)]
pub struct SwcEnvOptions<'a> {
    pub mode: Option<SwcEnvMode>,
    #[cfg_attr(rust_js, rust_js::name = "coreJs")]
    pub core_js: Option<&'a str>,
    pub skip: Option<&'a [&'a str]>,
    pub include: Option<&'a [&'a str]>,
    pub exclude: Option<&'a [&'a str]>,
    #[cfg_attr(rust_js, rust_js::name = "shippedProposals")]
    pub shipped_proposals: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "forceAllTransforms")]
    pub force_all_transforms: Option<bool>,
    pub debug: Option<bool>,
    pub loose: Option<bool>,
}

pub enum SwcEnvMode {
    #[cfg_attr(rust_js, rust_js::name = "usage")]
    Usage,
    #[cfg_attr(rust_js, rust_js::name = "entry")]
    Entry,
}

/// Subresource integrity, of its hash.
#[derive(Default)]
pub struct Sri {
    pub algorithm: Option<SubresourceIntegrityAlgorithm>,
}

pub enum SubresourceIntegrityAlgorithm {
    #[cfg_attr(rust_js, rust_js::name = "sha256")]
    Sha256,
    #[cfg_attr(rust_js, rust_js::name = "sha384")]
    Sha384,
    #[cfg_attr(rust_js, rust_js::name = "sha512")]
    Sha512,
}

/// A web vital, of `WEB_VITALS`.
pub enum WebVital {
    #[cfg_attr(rust_js, rust_js::name = "CLS")]
    Cls,
    #[cfg_attr(rust_js, rust_js::name = "FCP")]
    Fcp,
    #[cfg_attr(rust_js, rust_js::name = "FID")]
    Fid,
    #[cfg_attr(rust_js, rust_js::name = "INP")]
    Inp,
    #[cfg_attr(rust_js, rust_js::name = "LCP")]
    Lcp,
    #[cfg_attr(rust_js, rust_js::name = "TTFB")]
    Ttfb,
}

/// `false | "full" | "auto"`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackMemoryEviction {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "full")]
    Full,
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
}

pub enum TurbopackPluginRuntimeStrategy {
    #[cfg_attr(rust_js, rust_js::name = "workerThreads")]
    WorkerThreads,
    #[cfg_attr(rust_js, rust_js::name = "childProcesses")]
    ChildProcesses,
    #[cfg_attr(rust_js, rust_js::name = "forceWorkerThreads")]
    ForceWorkerThreads,
}

/// `"security" | "latest" | "experimental-future" | false`: each the
/// value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum AgentUpgrade {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "security")]
    Security,
    #[cfg_attr(rust_js, rust_js::name = "latest")]
    Latest,
    #[cfg_attr(rust_js, rust_js::name = "experimental-future")]
    ExperimentalFuture,
}

/// A directory outside Turbopack's root it reads.
pub struct TurbopackAdditionalRoot<'a> {
    pub path: &'a str,
    #[cfg_attr(rust_js, rust_js::name = "ignoreIfMissing")]
    pub ignore_if_missing: Option<bool>,
}

/// Turbopack's garbage collection, on, or of its options: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackGc {
    Bool(bool),
    Options(TurbopackGcOptions),
}

#[derive(Default)]
pub struct TurbopackGcOptions {
    #[cfg_attr(rust_js, rust_js::name = "minProgressMs")]
    pub min_progress_ms: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "rootTtlMs")]
    pub root_ttl_ms: Option<f64>,
}

/// Turbopack's minifying, on, or of each build: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TurbopackMinify {
    Bool(bool),
    Options(TurbopackMinifyOptions),
}

#[derive(Default)]
pub struct TurbopackMinifyOptions {
    pub server: Option<bool>,
    pub client: Option<bool>,
    pub edge: Option<bool>,
}

/// How Turbopack chunks a production build.
#[derive(Default)]
pub struct TurbopackChunking<'a> {
    #[cfg_attr(rust_js, rust_js::name = "firstPageLoadPriority")]
    pub first_page_load_priority: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "priorityRoutes")]
    pub priority_routes: Option<&'a [&'a RegExp]>,
    /// Routes chunked together, each cluster its patterns.
    pub clusters: Option<&'a [&'a [&'a RegExp]]>,
    #[cfg_attr(rust_js, rust_js::name = "priorityBoost")]
    pub priority_boost: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "requestCost")]
    pub request_cost: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "minChunkSize")]
    pub min_chunk_size: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maxChunkCountPerGroup")]
    pub max_chunk_count_per_group: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maxMergeChunkSize")]
    pub max_merge_chunk_size: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "generateComponentChunks")]
    pub generate_component_chunks: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "minComponentChunkSize")]
    pub min_component_chunk_size: Option<f64>,
}

pub enum TurbopackModuleIds {
    #[cfg_attr(rust_js, rust_js::name = "named")]
    Named,
    #[cfg_attr(rust_js, rust_js::name = "deterministic")]
    Deterministic,
}

/// MDX compiled by Rust's `mdxjs-rs`, on, or of its options: each the value
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum MdxRs<'a> {
    Bool(bool),
    Options(MdxRsOptions<'a>),
}

#[derive(Default)]
pub struct MdxRsOptions<'a> {
    pub development: Option<bool>,
    pub jsx: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "jsxRuntime")]
    pub jsx_runtime: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "jsxImportSource")]
    pub jsx_import_source: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "providerImportSource")]
    pub provider_import_source: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "mdxType")]
    pub mdx_type: Option<MdxType>,
}

pub enum MdxType {
    #[cfg_attr(rust_js, rust_js::name = "gfm")]
    Gfm,
    #[cfg_attr(rust_js, rust_js::name = "commonmark")]
    Commonmark,
}

/// `boolean | "incremental"`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ExperimentalPPRConfig {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "incremental")]
    Incremental,
}

#[derive(Default)]
pub struct ServerActions<'a> {
    /// The largest a Server Action's request body may be: `"1mb"`.
    #[cfg_attr(rust_js, rust_js::name = "bodySizeLimit")]
    pub body_size_limit: Option<SizeLimit<'a>>,
    /// The origins, besides the server's own, Server Actions are called
    /// from.
    #[cfg_attr(rust_js, rust_js::name = "allowedOrigins")]
    pub allowed_origins: Option<&'a [&'a str]>,
}

/// The LightningCSS features compiled, or not, whatever the browsers.
#[derive(Default)]
pub struct LightningCssFeatures<'a> {
    pub include: Option<&'a [LightningCssFeature]>,
    pub exclude: Option<&'a [LightningCssFeature]>,
}

/// A LightningCSS feature, of `LIGHTNINGCSS_FEATURE_NAMES`.
pub enum LightningCssFeature {
    #[cfg_attr(rust_js, rust_js::name = "nesting")]
    Nesting,
    #[cfg_attr(rust_js, rust_js::name = "not-selector-list")]
    NotSelectorList,
    #[cfg_attr(rust_js, rust_js::name = "dir-selector")]
    DirSelector,
    #[cfg_attr(rust_js, rust_js::name = "lang-selector-list")]
    LangSelectorList,
    #[cfg_attr(rust_js, rust_js::name = "is-selector")]
    IsSelector,
    #[cfg_attr(rust_js, rust_js::name = "text-decoration-thickness-percent")]
    TextDecorationThicknessPercent,
    #[cfg_attr(rust_js, rust_js::name = "media-interval-syntax")]
    MediaIntervalSyntax,
    #[cfg_attr(rust_js, rust_js::name = "media-range-syntax")]
    MediaRangeSyntax,
    #[cfg_attr(rust_js, rust_js::name = "custom-media-queries")]
    CustomMediaQueries,
    #[cfg_attr(rust_js, rust_js::name = "clamp-function")]
    ClampFunction,
    #[cfg_attr(rust_js, rust_js::name = "color-function")]
    ColorFunction,
    #[cfg_attr(rust_js, rust_js::name = "oklab-colors")]
    OklabColors,
    #[cfg_attr(rust_js, rust_js::name = "lab-colors")]
    LabColors,
    #[cfg_attr(rust_js, rust_js::name = "p3-colors")]
    P3Colors,
    #[cfg_attr(rust_js, rust_js::name = "hex-alpha-colors")]
    HexAlphaColors,
    #[cfg_attr(rust_js, rust_js::name = "space-separated-color-notation")]
    SpaceSeparatedColorNotation,
    #[cfg_attr(rust_js, rust_js::name = "font-family-system-ui")]
    FontFamilySystemUi,
    #[cfg_attr(rust_js, rust_js::name = "double-position-gradients")]
    DoublePositionGradients,
    #[cfg_attr(rust_js, rust_js::name = "vendor-prefixes")]
    VendorPrefixes,
    #[cfg_attr(rust_js, rust_js::name = "logical-properties")]
    LogicalProperties,
    #[cfg_attr(rust_js, rust_js::name = "light-dark")]
    LightDark,
    #[cfg_attr(rust_js, rust_js::name = "selectors")]
    Selectors,
    #[cfg_attr(rust_js, rust_js::name = "media-queries")]
    MediaQueries,
    #[cfg_attr(rust_js, rust_js::name = "colors")]
    Colors,
}

/// A test runner `next experimental-test` runs.
pub enum SupportedTestRunners {
    #[cfg_attr(rust_js, rust_js::name = "playwright")]
    Playwright,
}

#[derive(Default)]
pub struct InstantInsights {
    #[cfg_attr(rust_js, rust_js::name = "validationLevel")]
    pub validation_level: Option<ValidationLevel>,
}

pub enum ValidationLevel {
    #[cfg_attr(rust_js, rust_js::name = "warning")]
    Warning,
    #[cfg_attr(rust_js, rust_js::name = "manual-warning")]
    ManualWarning,
    #[cfg_attr(rust_js, rust_js::name = "experimental-error")]
    ExperimentalError,
    #[cfg_attr(rust_js, rust_js::name = "experimental-manual-error")]
    ExperimentalManualError,
}

/// The modules logged of a build that takes longer than this.
pub struct SlowModuleDetection {
    #[cfg_attr(rust_js, rust_js::name = "buildTimeThresholdMs")]
    pub build_time_threshold_ms: f64,
}

/// The browser's debug info in the terminal: on, of its level, or of its
/// options; each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum BrowserDebugInfoInTerminal {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "error")]
    Error,
    #[cfg_attr(rust_js, rust_js::name = "warn")]
    Warn,
    #[cfg_attr(rust_js, rust_js::name = "verbose")]
    Verbose,
    Options(BrowserDebugInfoOptions),
}

#[derive(Default)]
pub struct BrowserDebugInfoOptions {
    pub level: Option<DebugLevel>,
    #[cfg_attr(rust_js, rust_js::name = "depthLimit")]
    pub depth_limit: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "edgeLimit")]
    pub edge_limit: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "showSourceLocation")]
    pub show_source_location: Option<bool>,
}

pub enum DebugLevel {
    #[cfg_attr(rust_js, rust_js::name = "error")]
    Error,
    #[cfg_attr(rust_js, rust_js::name = "warn")]
    Warn,
    #[cfg_attr(rust_js, rust_js::name = "verbose")]
    Verbose,
}

/// `"error" | "warn"`.
pub enum ReportLevel {
    #[cfg_attr(rust_js, rust_js::name = "error")]
    Error,
    #[cfg_attr(rust_js, rust_js::name = "warn")]
    Warn,
}
