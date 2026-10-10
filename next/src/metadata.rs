//! [Metadata](https://nextjs.org/docs/app/api-reference/functions/generate-metadata):
//! a page's or a layout's `<head>`, its title, description, Open Graph
//! and icons, as `pub fn generateMetadata() -> Metadata<'static>` gives
//! it, and its `<meta name="viewport">`, as `generateViewport` does.
//!
//! Each field is `None` but what's given, left out, which a layout's page
//! inherits: TypeScript's `null`, which clears what a layout set, isn't
//! one yet. A union of TypeScript's is an untagged enum, each variant the
//! value itself (ADR 0214), `Title::Str("About")`; `OpenGraph` and
//! `Twitter`, a union of objects by their `type` and `card`, are one
//! struct of each one's fields, which Next.js reads by its `type`.

use core::marker::PhantomData;

use js::{Date, Dict, JsObject, Promise, Unknown};
use react::webapi::URL;

use crate::OneOrMany;

/// [`Metadata`](https://nextjs.org/docs/app/api-reference/functions/generate-metadata#metadata-fields):
/// a page's or a layout's metadata, as Next.js types it.
#[derive(Default)]
pub struct Metadata<'a> {
    /// The URL its relative URLs are of, `https://example.com`.
    #[cfg_attr(rust_js, rust_js::name = "metadataBase")]
    pub metadata_base: Option<StringOrUrl<'a>>,
    /// Its `<title>`, or a template its pages' titles fill.
    pub title: Option<Title<'a>>,
    pub description: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "applicationName")]
    pub application_name: Option<&'a str>,
    pub authors: Option<OneOrMany<'a, Author<'a>>>,
    pub generator: Option<&'a str>,
    pub keywords: Option<OneOrMany<'a, &'a str>>,
    /// `"no-referrer"`, `"origin"`, `"strict-origin-when-cross-origin"`..
    pub referrer: Option<&'a str>,
    /// Deprecated: [`Viewport`]'s.
    #[cfg_attr(rust_js, rust_js::name = "themeColor")]
    pub theme_color: Option<ThemeColor<'a>>,
    /// Deprecated: [`Viewport`]'s.
    #[cfg_attr(rust_js, rust_js::name = "colorScheme")]
    pub color_scheme: Option<&'a str>,
    /// Deprecated: [`Viewport`]'s.
    pub viewport: Option<MetadataViewport<'a>>,
    pub creator: Option<&'a str>,
    pub publisher: Option<&'a str>,
    /// Its `<meta name="robots">`: its text, or what it allows.
    pub robots: Option<MetadataRobots<'a>>,
    /// Its canonical URL, and the same page's in other languages.
    pub alternates: Option<AlternateURLs<'a>>,
    pub icons: Option<MetadataIcons<'a>>,
    /// Its web app manifest's URL.
    pub manifest: Option<StringOrUrl<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "openGraph")]
    pub open_graph: Option<OpenGraph<'a>>,
    pub twitter: Option<Twitter<'a>>,
    pub facebook: Option<Facebook<'a>>,
    pub pinterest: Option<Pinterest>,
    /// The search engines' site verification codes.
    pub verification: Option<Verification<'a>>,
    /// How iOS shows it, added to the home screen.
    #[cfg_attr(rust_js, rust_js::name = "appleWebApp")]
    pub apple_web_app: Option<MetadataAppleWebApp<'a>>,
    /// Which of its text Safari links, telephone numbers and the like.
    #[cfg_attr(rust_js, rust_js::name = "formatDetection")]
    pub format_detection: Option<FormatDetection>,
    /// Its iOS app's Smart App Banner.
    pub itunes: Option<ItunesApp<'a>>,
    pub r#abstract: Option<&'a str>,
    /// Its apps' App Links.
    #[cfg_attr(rust_js, rust_js::name = "appLinks")]
    pub app_links: Option<AppLinks<'a>>,
    pub archives: Option<OneOrMany<'a, &'a str>>,
    pub assets: Option<OneOrMany<'a, &'a str>>,
    pub bookmarks: Option<OneOrMany<'a, &'a str>>,
    /// The pages before and after it.
    pub pagination: Option<Pagination<'a>>,
    pub category: Option<&'a str>,
    pub classification: Option<&'a str>,
    /// Other `<meta>`s, by their names.
    pub other: Option<&'a Dict<OtherValue<'static>>>,
}

/// A text, or a [`URL`], `string | URL`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StringOrUrl<'a> {
    Str(&'a str),
    Url(&'a URL),
}

impl<'a> From<&'a str> for StringOrUrl<'a> {
    fn from(value: &'a str) -> Self {
        StringOrUrl::Str(value)
    }
}

impl<'a> From<&'a URL> for StringOrUrl<'a> {
    fn from(value: &'a URL) -> Self {
        StringOrUrl::Url(value)
    }
}

/// A text, or a number, `string | number`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum StringOrNumber<'a> {
    Str(&'a str),
    Number(f64),
}

/// An [`Metadata`]'s `other` value: a text, a number, or a list of them.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OtherValue<'a> {
    Str(&'a str),
    Number(f64),
    List(&'a [StringOrNumber<'a>]),
}

/// A title, `string | TemplateString`: each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Title<'a> {
    Str(&'a str),
    Template(TemplateString<'a>),
}

/// A title of its pages', as `TemplateString` types it: each the object
/// itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TemplateString<'a> {
    Default(DefaultTemplateString<'a>),
    AbsoluteTemplate(AbsoluteTemplateString<'a>),
    Absolute(AbsoluteString<'a>),
}

/// `{ default, template }`: the title where a page gives none, and the
/// template, `"%s | Acme"`, a page's fills.
pub struct DefaultTemplateString<'a> {
    pub default: &'a str,
    pub template: &'a str,
}

/// `{ absolute, template }`: a title no layout's template fills, and its
/// pages' template.
pub struct AbsoluteTemplateString<'a> {
    pub absolute: &'a str,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub template: Option<&'a str>,
}

/// `{ absolute }`: a title no layout's template fills.
pub struct AbsoluteString<'a> {
    pub absolute: &'a str,
}

/// A page's author: a name, and a URL.
#[derive(Default)]
pub struct Author<'a> {
    pub url: Option<StringOrUrl<'a>>,
    pub name: Option<&'a str>,
}

/// A theme color, `string | ThemeColorDescriptor | ThemeColorDescriptor[]`:
/// each the value itself.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum ThemeColor<'a> {
    Str(&'a str),
    Descriptor(ThemeColorDescriptor<'a>),
    List(&'a [ThemeColorDescriptor<'a>]),
}

/// A theme color, and the media query it's of.
pub struct ThemeColorDescriptor<'a> {
    pub color: &'a str,
    pub media: Option<&'a str>,
}

/// A [`Metadata`]'s `viewport`, `string | ViewportLayout`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum MetadataViewport<'a> {
    Str(&'a str),
    Layout(ViewportLayout<'a>),
}

/// A [`Metadata`]'s `robots`, `string | Robots`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum MetadataRobots<'a> {
    Str(&'a str),
    Robots(Robots<'a>),
}

/// What search engines may do with a page, its `<meta name="robots">`, and
/// Google's own.
#[derive(Default)]
pub struct Robots<'a> {
    pub index: Option<bool>,
    pub follow: Option<bool>,
    pub noarchive: Option<bool>,
    pub nosnippet: Option<bool>,
    pub noimageindex: Option<bool>,
    pub nocache: Option<bool>,
    pub notranslate: Option<bool>,
    pub indexifembedded: Option<bool>,
    pub nositelinkssearchbox: Option<bool>,
    pub unavailable_after: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "max-video-preview")]
    pub max_video_preview: Option<StringOrNumber<'a>>,
    /// `"none"`, `"standard"` or `"large"`.
    #[cfg_attr(rust_js, rust_js::name = "max-image-preview")]
    pub max_image_preview: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "max-snippet")]
    pub max_snippet: Option<f64>,
    /// Googlebot's, `<meta name="googlebot">`.
    #[cfg_attr(rust_js, rust_js::name = "googleBot")]
    pub google_bot: Option<GoogleBot<'a>>,
}

/// A [`Robots`]'s `googleBot`, `string | RobotsInfo`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum GoogleBot<'a> {
    Str(&'a str),
    Info(RobotsInfo<'a>),
}

/// What a search engine may do with a page.
#[derive(Default)]
pub struct RobotsInfo<'a> {
    pub index: Option<bool>,
    pub follow: Option<bool>,
    pub noarchive: Option<bool>,
    pub nosnippet: Option<bool>,
    pub noimageindex: Option<bool>,
    pub nocache: Option<bool>,
    pub notranslate: Option<bool>,
    pub indexifembedded: Option<bool>,
    pub nositelinkssearchbox: Option<bool>,
    pub unavailable_after: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "max-video-preview")]
    pub max_video_preview: Option<StringOrNumber<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "max-image-preview")]
    pub max_image_preview: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "max-snippet")]
    pub max_snippet: Option<f64>,
}

/// A page's canonical URL, and its other languages', media's and types'.
#[derive(Default)]
pub struct AlternateURLs<'a> {
    pub canonical: Option<Canonical<'a>>,
    /// Each language's, by its code, `"en-US"`, or `"x-default"`.
    pub languages: Option<&'a Languages<AlternateLinks<'static>>>,
    /// Each media query's.
    pub media: Option<&'a Dict<AlternateLinks<'static>>>,
    /// Each type's, `"application/rss+xml"`.
    pub types: Option<&'a Dict<AlternateLinks<'static>>>,
}

/// Each language's value, by its code, as `Languages<T>` types it.
pub type Languages<T> = Dict<T>;

/// An [`AlternateURLs`]'s `canonical`: a URL, or one titled.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Canonical<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptor(AlternateLinkDescriptor<'a>),
}

/// An alternate's URL, or its URLs, each titled.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum AlternateLinks<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptors(&'a [AlternateLinkDescriptor<'a>]),
}

/// A URL, and its title.
pub struct AlternateLinkDescriptor<'a> {
    pub title: Option<&'a str>,
    pub url: StringOrUrl<'a>,
}

/// A [`Metadata`]'s `icons`: a URL, a list of icons, or each kind's.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum MetadataIcons<'a> {
    Str(&'a str),
    Url(&'a URL),
    List(&'a [Icon<'a>]),
    Icons(Icons<'a>),
}

/// An icon: its URL, or its URL and how it's used.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Icon<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptor(IconDescriptor<'a>),
}

/// An icon's URL, and its `<link>`'s attributes.
pub struct IconDescriptor<'a> {
    pub url: StringOrUrl<'a>,
    pub r#type: Option<&'a str>,
    pub sizes: Option<&'a str>,
    pub color: Option<&'a str>,
    pub rel: Option<&'a str>,
    pub media: Option<&'a str>,
    /// `"high"`, `"low"` or `"auto"`.
    #[cfg_attr(rust_js, rust_js::name = "fetchPriority")]
    pub fetch_priority: Option<&'a str>,
}

/// Each kind of icon's.
#[derive(Default)]
pub struct Icons<'a> {
    pub icon: Option<OneOrMany<'a, Icon<'a>>>,
    pub shortcut: Option<OneOrMany<'a, Icon<'a>>>,
    pub apple: Option<OneOrMany<'a, Icon<'a>>>,
    pub other: Option<OneOrMany<'a, IconDescriptor<'a>>>,
}

/// A page's Open Graph, as `OpenGraph` types it: what each `type` has, one
/// struct of them all. `type` is `"website"`, `"article"`, `"book"`,
/// `"profile"`, `"music.song"`, `"music.album"`, `"music.playlist"`,
/// `"music.radio_station"`, `"video.movie"`, `"video.episode"`,
/// `"video.tv_show"` or `"video.other"`, or none.
#[derive(Default)]
pub struct OpenGraph<'a> {
    pub r#type: Option<&'a str>,
    /// `"a"`, `"an"`, `"the"`, `"auto"` or `""`.
    pub determiner: Option<&'a str>,
    pub title: Option<Title<'a>>,
    pub description: Option<&'a str>,
    pub emails: Option<OneOrMany<'a, &'a str>>,
    #[cfg_attr(rust_js, rust_js::name = "phoneNumbers")]
    pub phone_numbers: Option<OneOrMany<'a, &'a str>>,
    #[cfg_attr(rust_js, rust_js::name = "faxNumbers")]
    pub fax_numbers: Option<OneOrMany<'a, &'a str>>,
    #[cfg_attr(rust_js, rust_js::name = "siteName")]
    pub site_name: Option<&'a str>,
    pub locale: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "alternateLocale")]
    pub alternate_locale: Option<OneOrMany<'a, &'a str>>,
    pub images: Option<OneOrMany<'a, OgImage<'a>>>,
    pub audio: Option<OneOrMany<'a, OgAudio<'a>>>,
    pub videos: Option<OneOrMany<'a, OgVideo<'a>>>,
    pub url: Option<StringOrUrl<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "countryName")]
    pub country_name: Option<&'a str>,
    pub ttl: Option<f64>,
    /// An article's.
    #[cfg_attr(rust_js, rust_js::name = "publishedTime")]
    pub published_time: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "modifiedTime")]
    pub modified_time: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "expirationTime")]
    pub expiration_time: Option<&'a str>,
    /// An article's or a book's.
    pub authors: Option<OneOrMany<'a, StringOrUrl<'a>>>,
    pub section: Option<&'a str>,
    pub tags: Option<OneOrMany<'a, &'a str>>,
    /// A book's.
    pub isbn: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "releaseDate")]
    pub release_date: Option<&'a str>,
    /// A profile's.
    #[cfg_attr(rust_js, rust_js::name = "firstName")]
    pub first_name: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "lastName")]
    pub last_name: Option<&'a str>,
    pub username: Option<&'a str>,
    pub gender: Option<&'a str>,
    /// A song's, a movie's or an episode's, in seconds.
    pub duration: Option<f64>,
    /// A song's.
    pub albums: Option<OneOrMany<'a, OgAlbumLink<'a>>>,
    pub musicians: Option<OneOrMany<'a, StringOrUrl<'a>>>,
    /// An album's or a playlist's.
    pub songs: Option<OneOrMany<'a, OgSongLink<'a>>>,
    pub creators: Option<OneOrMany<'a, StringOrUrl<'a>>>,
    /// A movie's or an episode's.
    pub actors: Option<OneOrMany<'a, OgActorLink<'a>>>,
    pub directors: Option<OneOrMany<'a, StringOrUrl<'a>>>,
    pub writers: Option<OneOrMany<'a, StringOrUrl<'a>>>,
    /// An episode's.
    pub series: Option<StringOrUrl<'a>>,
}

/// An Open Graph image: its URL, or its URL and what it is.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OgImage<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptor(OgImageDescriptor<'a>),
}

/// An Open Graph image's URL, and its size and text.
pub struct OgImageDescriptor<'a> {
    pub url: StringOrUrl<'a>,
    #[cfg_attr(rust_js, rust_js::name = "secureUrl")]
    pub secure_url: Option<StringOrUrl<'a>>,
    pub alt: Option<&'a str>,
    pub r#type: Option<&'a str>,
    pub width: Option<StringOrNumber<'a>>,
    pub height: Option<StringOrNumber<'a>>,
}

/// An Open Graph audio: its URL, or its URL and type.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OgAudio<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptor(OgAudioDescriptor<'a>),
}

/// An Open Graph audio's URL, and its type.
pub struct OgAudioDescriptor<'a> {
    pub url: StringOrUrl<'a>,
    #[cfg_attr(rust_js, rust_js::name = "secureUrl")]
    pub secure_url: Option<StringOrUrl<'a>>,
    pub r#type: Option<&'a str>,
}

/// An Open Graph video: its URL, or its URL, type and size.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OgVideo<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptor(OgVideoDescriptor<'a>),
}

/// An Open Graph video's URL, and its type and size.
pub struct OgVideoDescriptor<'a> {
    pub url: StringOrUrl<'a>,
    #[cfg_attr(rust_js, rust_js::name = "secureUrl")]
    pub secure_url: Option<StringOrUrl<'a>>,
    pub r#type: Option<&'a str>,
    pub width: Option<StringOrNumber<'a>>,
    pub height: Option<StringOrNumber<'a>>,
}

/// A song's album: its URL, or its URL, disc and track.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OgAlbumLink<'a> {
    Str(&'a str),
    Url(&'a URL),
    Album(OgAlbum<'a>),
}

/// An album's URL, and the song's disc and track on it.
pub struct OgAlbum<'a> {
    pub url: StringOrUrl<'a>,
    pub disc: Option<f64>,
    pub track: Option<f64>,
}

/// An album's song: its URL, or its URL, disc and track.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OgSongLink<'a> {
    Str(&'a str),
    Url(&'a URL),
    Song(OgSong<'a>),
}

/// A song's URL, and its disc and track.
pub struct OgSong<'a> {
    pub url: StringOrUrl<'a>,
    pub disc: Option<f64>,
    pub track: Option<f64>,
}

/// A movie's actor: their URL, or their URL and role.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum OgActorLink<'a> {
    Str(&'a str),
    Url(&'a URL),
    Actor(OgActor<'a>),
}

/// An actor's URL, and their role.
pub struct OgActor<'a> {
    pub url: StringOrUrl<'a>,
    pub role: Option<&'a str>,
}

/// A page's X (Twitter) card, as `Twitter` types it: what each `card` has,
/// one struct of them all. `card` is `"summary"`,
/// `"summary_large_image"`, `"player"` or `"app"`, or none.
#[derive(Default)]
pub struct Twitter<'a> {
    pub card: Option<&'a str>,
    pub site: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "siteId")]
    pub site_id: Option<&'a str>,
    pub creator: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "creatorId")]
    pub creator_id: Option<&'a str>,
    pub description: Option<&'a str>,
    pub title: Option<Title<'a>>,
    pub images: Option<OneOrMany<'a, TwitterImage<'a>>>,
    /// A `"player"` card's.
    pub players: Option<OneOrMany<'a, TwitterPlayerDescriptor<'a>>>,
    /// An `"app"` card's.
    pub app: Option<TwitterAppDescriptor<'a>>,
}

/// A card's image: its URL, or its URL and what it is.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum TwitterImage<'a> {
    Str(&'a str),
    Url(&'a URL),
    Descriptor(TwitterImageDescriptor<'a>),
}

/// A card's image's URL, and its text and size.
pub struct TwitterImageDescriptor<'a> {
    pub url: StringOrUrl<'a>,
    pub alt: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "secureUrl")]
    pub secure_url: Option<StringOrUrl<'a>>,
    pub r#type: Option<&'a str>,
    pub width: Option<StringOrNumber<'a>>,
    pub height: Option<StringOrNumber<'a>>,
}

/// A player card's player.
pub struct TwitterPlayerDescriptor<'a> {
    #[cfg_attr(rust_js, rust_js::name = "playerUrl")]
    pub player_url: StringOrUrl<'a>,
    #[cfg_attr(rust_js, rust_js::name = "streamUrl")]
    pub stream_url: StringOrUrl<'a>,
    pub width: f64,
    pub height: f64,
}

/// An app card's app, on each store.
pub struct TwitterAppDescriptor<'a> {
    pub id: TwitterAppIds<'a>,
    pub url: Option<TwitterAppUrls<'a>>,
    pub name: Option<&'a str>,
}

/// An app's id on each store.
#[derive(Default)]
pub struct TwitterAppIds<'a> {
    pub iphone: Option<StringOrNumber<'a>>,
    pub ipad: Option<StringOrNumber<'a>>,
    pub googleplay: Option<&'a str>,
}

/// An app's URL on each store.
#[derive(Default)]
pub struct TwitterAppUrls<'a> {
    pub iphone: Option<StringOrUrl<'a>>,
    pub ipad: Option<StringOrUrl<'a>>,
    pub googleplay: Option<StringOrUrl<'a>>,
}

/// A page's Facebook app, or its admins, as `Facebook` types it: one of
/// the two.
#[derive(Default)]
pub struct Facebook<'a> {
    #[cfg_attr(rust_js, rust_js::name = "appId")]
    pub app_id: Option<&'a str>,
    pub admins: Option<OneOrMany<'a, &'a str>>,
}

/// A page's Pinterest Rich Pin.
pub struct Pinterest {
    #[cfg_attr(rust_js, rust_js::name = "richPin")]
    pub rich_pin: RichPin,
}

/// A [`Pinterest`]'s `richPin`, `string | boolean`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum RichPin {
    Str(&'static str),
    Bool(bool),
}

/// Search engines' site verification codes, each one or more.
#[derive(Default)]
pub struct Verification<'a> {
    pub google: Option<OneOrMany<'a, StringOrNumber<'a>>>,
    pub yahoo: Option<OneOrMany<'a, StringOrNumber<'a>>>,
    pub yandex: Option<OneOrMany<'a, StringOrNumber<'a>>>,
    pub me: Option<OneOrMany<'a, StringOrNumber<'a>>>,
    /// Another's, by its name.
    pub other: Option<&'a Dict<OneOrMany<'static, StringOrNumber<'static>>>>,
}

/// A [`Metadata`]'s `appleWebApp`, `boolean | AppleWebApp`.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum MetadataAppleWebApp<'a> {
    Bool(bool),
    AppleWebApp(AppleWebApp<'a>),
}

/// How iOS shows a page added to the home screen.
#[derive(Default)]
pub struct AppleWebApp<'a> {
    pub capable: Option<bool>,
    pub title: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "startupImage")]
    pub startup_image: Option<OneOrMany<'a, AppleImage<'a>>>,
    /// `"default"`, `"black"` or `"black-translucent"`.
    #[cfg_attr(rust_js, rust_js::name = "statusBarStyle")]
    pub status_bar_style: Option<&'a str>,
}

/// A startup image: its URL, or its URL and media query.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum AppleImage<'a> {
    Str(&'a str),
    Descriptor(AppleImageDescriptor<'a>),
}

/// A startup image's URL, and its media query.
pub struct AppleImageDescriptor<'a> {
    pub url: &'a str,
    pub media: Option<&'a str>,
}

/// Which of a page's text Safari links.
#[derive(Default)]
pub struct FormatDetection {
    pub telephone: Option<bool>,
    pub date: Option<bool>,
    pub address: Option<bool>,
    pub email: Option<bool>,
    pub url: Option<bool>,
}

/// An iOS app's Smart App Banner.
pub struct ItunesApp<'a> {
    #[cfg_attr(rust_js, rust_js::name = "appId")]
    pub app_id: &'a str,
    #[cfg_attr(rust_js, rust_js::name = "appArgument")]
    pub app_argument: Option<&'a str>,
}

/// A page's App Links, each platform's.
#[derive(Default)]
pub struct AppLinks<'a> {
    pub ios: Option<OneOrMany<'a, AppLinksApple<'a>>>,
    pub iphone: Option<OneOrMany<'a, AppLinksApple<'a>>>,
    pub ipad: Option<OneOrMany<'a, AppLinksApple<'a>>>,
    pub android: Option<OneOrMany<'a, AppLinksAndroid<'a>>>,
    pub windows_phone: Option<OneOrMany<'a, AppLinksWindows<'a>>>,
    pub windows: Option<OneOrMany<'a, AppLinksWindows<'a>>>,
    pub windows_universal: Option<OneOrMany<'a, AppLinksWindows<'a>>>,
    pub web: Option<OneOrMany<'a, AppLinksWeb<'a>>>,
}

/// An iOS app's link.
pub struct AppLinksApple<'a> {
    pub url: StringOrUrl<'a>,
    pub app_store_id: Option<StringOrNumber<'a>>,
    pub app_name: Option<&'a str>,
}

/// An Android app's link.
pub struct AppLinksAndroid<'a> {
    pub package: &'a str,
    pub url: Option<StringOrUrl<'a>>,
    pub class: Option<&'a str>,
    pub app_name: Option<&'a str>,
}

/// A Windows app's link.
pub struct AppLinksWindows<'a> {
    pub url: StringOrUrl<'a>,
    pub app_id: Option<&'a str>,
    pub app_name: Option<&'a str>,
}

/// The web's link, where there's no app.
pub struct AppLinksWeb<'a> {
    pub url: StringOrUrl<'a>,
    pub should_fallback: Option<bool>,
}

/// The pages before and after a page.
#[derive(Default)]
pub struct Pagination<'a> {
    pub previous: Option<StringOrUrl<'a>>,
    pub next: Option<StringOrUrl<'a>>,
}

/// [`Viewport`](https://nextjs.org/docs/app/api-reference/functions/generate-viewport):
/// a page's `<meta name="viewport">`, theme color and color scheme.
#[derive(Default)]
pub struct Viewport<'a> {
    pub width: Option<StringOrNumber<'a>>,
    pub height: Option<StringOrNumber<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "initialScale")]
    pub initial_scale: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "minimumScale")]
    pub minimum_scale: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maximumScale")]
    pub maximum_scale: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "userScalable")]
    pub user_scalable: Option<bool>,
    /// `"auto"`, `"cover"` or `"contain"`.
    #[cfg_attr(rust_js, rust_js::name = "viewportFit")]
    pub viewport_fit: Option<&'a str>,
    /// `"resizes-visual"`, `"resizes-content"` or `"overlays-content"`.
    #[cfg_attr(rust_js, rust_js::name = "interactiveWidget")]
    pub interactive_widget: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "themeColor")]
    pub theme_color: Option<ThemeColor<'a>>,
    /// `"normal"`, `"light"`, `"dark"`, `"light dark"`, `"dark light"` or
    /// `"only light"`.
    #[cfg_attr(rust_js, rust_js::name = "colorScheme")]
    pub color_scheme: Option<&'a str>,
}

/// A viewport's layout, as `ViewportLayout` types it: [`Viewport`]'s, and
/// a [`Metadata`]'s deprecated `viewport`.
#[derive(Default)]
pub struct ViewportLayout<'a> {
    pub width: Option<StringOrNumber<'a>>,
    pub height: Option<StringOrNumber<'a>>,
    #[cfg_attr(rust_js, rust_js::name = "initialScale")]
    pub initial_scale: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "minimumScale")]
    pub minimum_scale: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "maximumScale")]
    pub maximum_scale: Option<f64>,
    #[cfg_attr(rust_js, rust_js::name = "userScalable")]
    pub user_scalable: Option<bool>,
    #[cfg_attr(rust_js, rust_js::name = "viewportFit")]
    pub viewport_fit: Option<&'a str>,
    #[cfg_attr(rust_js, rust_js::name = "interactiveWidget")]
    pub interactive_widget: Option<&'a str>,
}

/// The metadata a page's layouts resolved, what `generateMetadata` is
/// given second, read as JS has it: a field of each, `None` where it's
/// `null`.
pub struct ResolvedMetadata(PhantomData<JsObject>);

impl ResolvedMetadata {
    /// Its title, `{ absolute, template }`.
    #[cfg_attr(rust_js, rust_js::link_name = "get title")]
    pub fn title(&self) -> Option<&'static AbsoluteTemplate> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get description")]
    pub fn description(&self) -> Option<String> {
        unreachable!()
    }

    /// Its Open Graph, its images resolved each to `{ url, .. }`.
    #[cfg_attr(rust_js, rust_js::link_name = "get openGraph")]
    pub fn open_graph(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get twitter")]
    pub fn twitter(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get keywords")]
    pub fn keywords(&self) -> Option<Vec<String>> {
        unreachable!()
    }

    /// Any of its fields, by its name, as it's resolved.
    #[cfg_attr(rust_js, rust_js::link_name = "get []")]
    pub fn get(&self, name: &str) -> Option<&'static Unknown> {
        unreachable!()
    }
}

/// A resolved title: what it is, and its pages' template.
pub struct AbsoluteTemplate(PhantomData<JsObject>);

impl AbsoluteTemplate {
    #[cfg_attr(rust_js, rust_js::link_name = "get absolute")]
    pub fn absolute(&self) -> String {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get template")]
    pub fn template(&self) -> Option<String> {
        unreachable!()
    }
}

/// What `generateMetadata` is given second: its layouts' metadata, as it
/// resolves.
pub type ResolvingMetadata = Promise<&'static ResolvedMetadata>;

/// The viewport a page's layouts resolved, what `generateViewport` is
/// given second.
pub struct ResolvedViewport(PhantomData<JsObject>);

impl ResolvedViewport {
    #[cfg_attr(rust_js, rust_js::link_name = "get width")]
    pub fn width(&self) -> Option<StringOrNumber<'static>> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get height")]
    pub fn height(&self) -> Option<StringOrNumber<'static>> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get initialScale")]
    pub fn initial_scale(&self) -> Option<f64> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get minimumScale")]
    pub fn minimum_scale(&self) -> Option<f64> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get maximumScale")]
    pub fn maximum_scale(&self) -> Option<f64> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get userScalable")]
    pub fn user_scalable(&self) -> Option<bool> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get viewportFit")]
    pub fn viewport_fit(&self) -> Option<String> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get interactiveWidget")]
    pub fn interactive_widget(&self) -> Option<String> {
        unreachable!()
    }

    /// Its theme colors, each of its media query; `None` of `null`.
    #[cfg_attr(rust_js, rust_js::link_name = "get themeColor")]
    pub fn theme_color(&self) -> Option<Vec<ThemeColorDescriptor<'static>>> {
        unreachable!()
    }

    /// `"normal"`, `"light"`, `"dark"`, `"light dark"`, `"dark light"` or `"only light"`; `None` of `null`.
    #[cfg_attr(rust_js, rust_js::link_name = "get colorScheme")]
    pub fn color_scheme(&self) -> Option<String> {
        unreachable!()
    }
}

/// What `generateViewport` is given second.
pub type ResolvingViewport = Promise<&'static ResolvedViewport>;

/// What `app/robots.rs`, `app/sitemap.rs` and `app/manifest.rs` give, as
/// `MetadataRoute` names them.
#[allow(non_snake_case)]
pub mod MetadataRoute {
    use super::*;

    /// `app/robots.rs`'s: the rules of `robots.txt`.
    pub struct Robots<'a> {
        pub rules: RobotsRules<'a>,
        pub sitemap: Option<OneOrMany<'a, &'a str>>,
        pub host: Option<&'a str>,
    }

    /// A [`Robots`]'s rules: one, or each user agent's.
    #[cfg_attr(rust_js, rust_js::untagged)]
    pub enum RobotsRules<'a> {
        One(RobotsRule<'a>),
        Each(&'a [RobotsRule<'a>]),
    }

    /// What a user agent, or every one, may crawl.
    #[derive(Default)]
    pub struct RobotsRule<'a> {
        /// Needed of each of a list.
        #[cfg_attr(rust_js, rust_js::name = "userAgent")]
        pub user_agent: Option<OneOrMany<'a, &'a str>>,
        pub allow: Option<OneOrMany<'a, &'a str>>,
        pub disallow: Option<OneOrMany<'a, &'a str>>,
        #[cfg_attr(rust_js, rust_js::name = "crawlDelay")]
        pub crawl_delay: Option<f64>,
        pub other: Option<&'a Dict<OtherValue<'static>>>,
    }

    /// `app/sitemap.rs`'s: each URL of `sitemap.xml`.
    pub type Sitemap<'a> = Vec<SitemapEntry<'a>>;

    /// A URL of a [`Sitemap`].
    #[derive(Default)]
    pub struct SitemapEntry<'a> {
        pub url: &'a str,
        #[cfg_attr(rust_js, rust_js::name = "lastModified")]
        pub last_modified: Option<LastModified<'a>>,
        /// `"always"`, `"hourly"`, `"daily"`, `"weekly"`, `"monthly"`,
        /// `"yearly"` or `"never"`.
        #[cfg_attr(rust_js, rust_js::name = "changeFrequency")]
        pub change_frequency: Option<&'a str>,
        pub priority: Option<f64>,
        pub alternates: Option<SitemapAlternates<'a>>,
        pub images: Option<&'a [&'a str]>,
        pub videos: Option<&'a [Videos<'a>]>,
    }

    /// A [`SitemapEntry`]'s `lastModified`, `string | Date`.
    #[cfg_attr(rust_js, rust_js::untagged)]
    pub enum LastModified<'a> {
        Str(&'a str),
        Date(&'a Date),
    }

    /// A [`SitemapEntry`]'s URL in other languages.
    #[derive(Default)]
    pub struct SitemapAlternates<'a> {
        pub languages: Option<&'a Languages<&'static str>>,
    }

    /// A video of a [`SitemapEntry`].
    pub struct Videos<'a> {
        pub title: &'a str,
        pub thumbnail_loc: &'a str,
        pub description: &'a str,
        pub content_loc: Option<&'a str>,
        pub player_loc: Option<&'a str>,
        pub duration: Option<f64>,
        pub expiration_date: Option<LastModified<'a>>,
        pub rating: Option<f64>,
        pub view_count: Option<f64>,
        pub publication_date: Option<LastModified<'a>>,
        /// `"yes"` or `"no"`.
        pub family_friendly: Option<&'a str>,
        pub restriction: Option<Restriction<'a>>,
        pub platform: Option<Restriction<'a>>,
        pub requires_subscription: Option<&'a str>,
        pub uploader: Option<Uploader<'a>>,
        pub live: Option<&'a str>,
        pub tag: Option<&'a str>,
    }

    /// Where a video may be shown: `"allow"` or `"deny"` of `content`.
    pub struct Restriction<'a> {
        pub relationship: &'a str,
        pub content: &'a str,
    }

    /// A video's uploader.
    #[derive(Default)]
    pub struct Uploader<'a> {
        pub info: Option<&'a str>,
        pub content: Option<&'a str>,
    }

    /// `app/manifest.rs`'s: the web app manifest.
    #[derive(Default)]
    pub struct Manifest<'a> {
        pub background_color: Option<&'a str>,
        pub categories: Option<&'a [&'a str]>,
        pub description: Option<&'a str>,
        /// `"ltr"`, `"rtl"` or `"auto"`.
        pub dir: Option<&'a str>,
        /// `"fullscreen"`, `"standalone"`, `"minimal-ui"` or `"browser"`.
        pub display: Option<&'a str>,
        pub display_override: Option<&'a [&'a str]>,
        pub file_handlers: Option<&'a [FileHandler<'a>]>,
        pub icons: Option<&'a [ManifestIcon<'a>]>,
        pub id: Option<&'a str>,
        pub lang: Option<&'a str>,
        pub launch_handler: Option<LaunchHandler<'a>>,
        pub name: Option<&'a str>,
        pub orientation: Option<&'a str>,
        pub prefer_related_applications: Option<bool>,
        pub protocol_handlers: Option<&'a [ProtocolHandler<'a>]>,
        pub related_applications: Option<&'a [RelatedApplication<'a>]>,
        pub scope: Option<&'a str>,
        pub screenshots: Option<&'a [Screenshot<'a>]>,
        pub share_target: Option<ShareTarget<'a>>,
        pub short_name: Option<&'a str>,
        pub shortcuts: Option<&'a [Shortcut<'a>]>,
        pub start_url: Option<&'a str>,
        pub theme_color: Option<&'a str>,
    }

    /// A file type the app opens.
    pub struct FileHandler<'a> {
        pub action: &'a str,
        /// Each MIME type's extensions.
        pub accept: &'a Dict<Vec<&'static str>>,
    }

    /// A manifest's icon.
    pub struct ManifestIcon<'a> {
        pub src: &'a str,
        pub r#type: Option<&'a str>,
        pub sizes: Option<&'a str>,
        /// `"any"`, `"maskable"` or `"monochrome"`.
        pub purpose: Option<&'a str>,
    }

    /// How the app is launched: `"auto"`, `"focus-existing"`,
    /// `"navigate-existing"` or `"navigate-new"`, or a list of them.
    pub struct LaunchHandler<'a> {
        pub client_mode: OneOrMany<'a, &'a str>,
    }

    /// A protocol the app handles.
    pub struct ProtocolHandler<'a> {
        pub protocol: &'a str,
        pub url: &'a str,
    }

    /// A native app of the same.
    pub struct RelatedApplication<'a> {
        pub platform: &'a str,
        pub url: &'a str,
        pub id: Option<&'a str>,
    }

    /// A screenshot of the app.
    pub struct Screenshot<'a> {
        /// `"narrow"` or `"wide"`.
        pub form_factor: Option<&'a str>,
        pub label: Option<&'a str>,
        pub platform: Option<&'a str>,
        pub src: &'a str,
        pub r#type: Option<&'a str>,
        pub sizes: Option<&'a str>,
    }

    /// What the app takes, shared to it.
    pub struct ShareTarget<'a> {
        pub action: &'a str,
        /// `"get"`, `"post"`, `"GET"` or `"POST"`.
        pub method: Option<&'a str>,
        pub enctype: Option<&'a str>,
        pub params: ShareParams<'a>,
    }

    /// A [`ShareTarget`]'s parameters' names.
    #[derive(Default)]
    pub struct ShareParams<'a> {
        pub title: Option<&'a str>,
        pub text: Option<&'a str>,
        pub url: Option<&'a str>,
        pub files: Option<OneOrMany<'a, ShareFile<'a>>>,
    }

    /// A file a [`ShareTarget`] takes.
    pub struct ShareFile<'a> {
        pub name: &'a str,
        pub accept: OneOrMany<'a, &'a str>,
    }

    /// A shortcut of the app's.
    pub struct Shortcut<'a> {
        pub name: &'a str,
        pub short_name: Option<&'a str>,
        pub description: Option<&'a str>,
        pub url: &'a str,
        pub icons: Option<&'a [ManifestIcon<'a>]>,
    }
}
