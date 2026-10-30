//! Picture addresses, retained resources and natural image sizing.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::Engine as _;
use gpui::{
    AnyElement, App, Asset, ElementId, Hsla, Image, ImageAssetLoader, ImageFormat,
    InteractiveElement, IntoElement, ParentElement, Pixels, RenderImage, RenderOnce, Resource,
    SMOOTH_SVG_SCALE_FACTOR, SharedString, Styled, Window, div, img,
};

use crate::{
    i18n,
    primitives::{Icon, IconName, hold},
    theme::{ActiveTheme, IconSize, Radius},
};

/// A picture's bytes, hashed once: frames compare it.
#[derive(Clone, Debug)]
pub struct HeldPicture {
    format: ImageFormat,
    bytes: Arc<[u8]>,
    digest: u64,
}

impl HeldPicture {
    /// A picture of `format` from `bytes`, hashed.
    pub fn new(format: ImageFormat, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Self {
            format,
            bytes,
            digest: hasher.finish(),
        }
    }
}

impl PartialEq for HeldPicture {
    fn eq(&self, other: &Self) -> bool {
        self.format == other.format
            && self.bytes.len() == other.bytes.len()
            && self.digest == other.digest
    }
}

impl Eq for HeldPicture {}

impl Hash for HeldPicture {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.format, self.bytes.len(), self.digest).hash(state);
    }
}

/// Held pictures by the address their blocks name, or why not.
pub type Pictures = HashMap<String, Result<HeldPicture, SharedString>>;

/// Where a picture's bytes come from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Origin {
    File(PathBuf),
    /// Bytes inline in the address, and their type.
    Data(HeldPicture),
    Web,
    /// A relative path with no folder to read it in.
    Loose,
    /// An address that names no picture, and why.
    Bad(SharedString),
}

/// `%XX` decoded; a broken escape stays as written.
fn decoded(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let (mut out, mut at) = (Vec::with_capacity(bytes.len()), 0);
    while at < bytes.len() {
        let pair = bytes.get(at + 1..at + 3);
        let hex =
            pair.and_then(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(byte)) => (out.push(byte), at += 3),
            (byte, _) => (out.push(byte), at += 1),
        };
    }
    out
}

/// A path written in a link, `%XX` decoded.
fn path(written: &str) -> Result<PathBuf, SharedString> {
    String::from_utf8(decoded(written))
        .map(PathBuf::from)
        .map_err(|_| "the path is not UTF-8".into())
}

/// Where `url` points, read from the folder `base`.
pub(crate) fn origin(url: &str, base: Option<&Path>) -> Origin {
    let url = url.trim();
    let (scheme, rest) = url.split_once(':').unwrap_or(("", url));
    if ((scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        && rest.starts_with("//"))
        || url.starts_with("//")
    {
        return Origin::Web;
    }
    if scheme.eq_ignore_ascii_case("data") {
        return inline(url);
    }
    if scheme.eq_ignore_ascii_case("file") {
        return file(rest);
    }
    if scheme.len() > 1
        && scheme
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "+-.".contains(ch))
    {
        return Origin::Bad(format!("{url} is no picture address").into());
    }
    let written = url.split(['?', '#']).next().unwrap_or_default();
    match path(written) {
        Ok(path) if path.is_absolute() => Origin::File(path),
        Ok(path) => base.map_or(Origin::Loose, |base| Origin::File(base.join(path))),
        Err(why) => Origin::Bad(why),
    }
}

/// A `file:` address after its scheme: host, path, no query.
fn file(rest: &str) -> Origin {
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (host, written) = match rest.strip_prefix("//") {
        Some(after) => after.split_at(after.find('/').unwrap_or(after.len())),
        None => ("", rest),
    };
    let local = host.is_empty() || host.eq_ignore_ascii_case("localhost");
    let written = match (local, written.as_bytes()) {
        // Windows writes file:///C:/…
        (true, [b'/', _, b':', ..]) if cfg!(windows) => written[1..].to_string(),
        (true, _) => written.to_string(),
        (false, _) if cfg!(windows) => format!("//{host}{written}"),
        (false, _) => return Origin::Bad(format!("files on {host} are not read").into()),
    };
    path(&written).map_or_else(Origin::Bad, Origin::File)
}

/// A `data:` address: base64 bytes of an image type.
fn inline(url: &str) -> Origin {
    let Some((head, payload)) = url[5..].split_once(',') else {
        return Origin::Bad("the data address has no comma".into());
    };
    let Some(mime) = head.strip_suffix(";base64") else {
        return Origin::Bad("only base64 data addresses draw".into());
    };
    let Some(format) = ImageFormat::from_mime_type(&mime.to_ascii_lowercase()) else {
        return Origin::Bad(format!("{mime} is no image type").into());
    };
    match base64::engine::general_purpose::STANDARD.decode(payload.trim()) {
        Ok(bytes) => Origin::Data(HeldPicture::new(format, bytes)),
        Err(error) => Origin::Bad(format!("{error}").into()),
    }
}

/// Reuse resolution and inline hashing until an element's URL or folder changes.
#[derive(Default)]
struct OriginCache(Option<Resolved>);

struct Resolved {
    url: String,
    base: Option<PathBuf>,
    origin: Origin,
}

impl OriginCache {
    fn get(&mut self, url: &str, base: Option<&Path>) -> Origin {
        if let Some(saved) = &self.0
            && saved.url == url
            && saved.base.as_deref() == base
        {
            return saved.origin.clone();
        }
        let origin = origin(url, base);
        self.0 = Some(Resolved {
            url: url.to_owned(),
            base: base.map(Path::to_path_buf),
            origin: origin.clone(),
        });
        origin
    }
}

/// Held bytes decoded as gpui decodes files.
enum Holder {}

impl Asset for Holder {
    type Source = HeldPicture;
    type Output = Result<Arc<RenderImage>, SharedString>;

    fn load(
        held: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let renderer = cx.svg_renderer();
        async move {
            Image::from_bytes(held.format, held.bytes.to_vec())
                .to_image_data(renderer)
                .map_err(|error| format!("{error:#}").into())
        }
    }
}

/// The picture at `url`, natural size up to the column.
#[derive(IntoElement)]
pub(crate) struct Picture {
    pub id: ElementId,
    pub alt: String,
    pub url: String,
    /// The folder relative addresses start from.
    pub base: Option<Arc<Path>>,
    pub held: Option<Result<HeldPicture, SharedString>>,
}

impl RenderOnce for Picture {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        picture(self, window, cx)
    }
}

fn picture(shown: Picture, window: &mut Window, cx: &mut App) -> AnyElement {
    let Picture {
        id,
        alt,
        url,
        base,
        held,
    } = shown;
    let (alt, url) = (alt.as_str(), url.as_str());
    let colors = cx.theme().colors.clone();
    let origin = match held {
        Some(Ok(held)) => Origin::Data(held),
        Some(Err(why)) => Origin::Bad(why),
        None => {
            let cache =
                window.use_keyed_state((id.clone(), "origin"), cx, |_, _| OriginCache::default());
            cache.update(cx, |cache, _| cache.get(url, base.as_deref()))
        }
    };
    let (loaded, svg) = match origin {
        Origin::File(path) => {
            let svg = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"));
            let resource = Resource::Path(path.into());
            let image = |loaded: &Result<Arc<RenderImage>, _>| loaded.as_ref().ok().cloned();
            let loaded = hold::<ImageAssetLoader>(id, &resource, image, window, cx)
                .map(|loaded| loaded.map_err(|error| SharedString::from(error.to_string())));
            (loaded, svg)
        }
        Origin::Data(held) => {
            let svg = held.format == ImageFormat::Svg;
            let image = |loaded: &Result<Arc<RenderImage>, _>| loaded.as_ref().ok().cloned();
            (hold::<Holder>(id, &held, image, window, cx), svg)
        }
        Origin::Web => {
            let said = i18n::text(cx, "markdown.picture.web", &[]);
            return note(alt, said, colors.fg_subtle, cx);
        }
        Origin::Loose => {
            let said = i18n::text(cx, "markdown.picture.folder", &[]);
            return note(alt, said, colors.fg_subtle, cx);
        }
        Origin::Bad(why) => return note(alt, why, colors.danger, cx),
    };
    drawn(loaded, svg, alt, url, cx)
}

/// A loaded picture at its own width, or why not.
fn drawn(
    loaded: Option<Result<Arc<RenderImage>, SharedString>>,
    svg: bool,
    alt: &str,
    url: &str,
    cx: &App,
) -> AnyElement {
    let colors = cx.theme().colors.clone();
    match loaded {
        Some(Ok(image)) => {
            let natural = image.size(0);
            // The picture's own height follows its ratio.
            let smooth = if svg { SMOOTH_SVG_SCALE_FACTOR } else { 1.0 };
            let width = i32::from(natural.width) as f32 / smooth;
            div()
                .debug_selector(|| "markdown-picture".into())
                .w(Pixels::from(width))
                .max_w_full()
                .child(img(image).size_full())
                .into_any_element()
        }
        Some(Err(why)) => {
            log::warn!("markdown: a picture did not load");
            note(alt, why, colors.danger, cx)
        }
        None => note(alt, pending_address(url), colors.fg_subtle, cx),
    }
}

fn pending_address(url: &str) -> SharedString {
    if let Some((head, _)) = url.trim().split_once(',')
        && head
            .get(..5)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
    {
        return format!("{head},…").into();
    }
    url.to_owned().into()
}

/// A picture not drawn: its alt text and why.
fn note(alt: &str, said: SharedString, tone: Hsla, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(colors.border)
        .text_color(colors.fg_muted)
        .child(
            Icon::new(IconName::Image)
                .size(IconSize::Sm)
                .color(colors.fg_subtle),
        )
        .child(div().flex_1().child(alt.to_string()))
        .child(div().text_color(tone).child(said))
        .into_any_element()
}

#[cfg(test)]
mod tests;
