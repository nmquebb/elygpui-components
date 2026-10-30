//! Address resolution, retained bytes and rendered source replacement.

use super::{HeldPicture, Origin, OriginCache, Picture, origin, pending_address};
use crate::theme::Theme;
use base64::Engine as _;
use gpui::{
    Context, ImageFormat, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext,
    VisualTestContext, Window, div, px, size,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[test]
fn addresses_resolve_against_the_folder() {
    let base = Path::new("/notes");
    assert_eq!(
        origin("assets/a%20b.png?x#y", Some(base)),
        Origin::File(base.join("assets/a b.png"))
    );
    assert_eq!(origin("assets/a.png", None), Origin::Loose);
    assert_eq!(origin("https://e.x/a.png", Some(base)), Origin::Web);
    assert_eq!(origin("//e.x/a.png", Some(base)), Origin::Web);
    assert!(matches!(origin("mailto:a@b", Some(base)), Origin::Bad(_)));
    let absolute = std::env::current_dir().expect("a dir").join("a.png");
    assert_eq!(
        origin(&absolute.display().to_string(), Some(base)),
        Origin::File(absolute)
    );
}

#[test]
fn file_addresses_decode_without_query_or_fragment() {
    for url in [
        "file:///tmp/a%23b.png",
        "file://localhost/tmp/a%23b.png#x",
        "FILE:/tmp/a%23b.png?y",
    ] {
        assert_eq!(
            origin(url, None),
            Origin::File(PathBuf::from("/tmp/a#b.png")),
            "{url}"
        );
    }
    if cfg!(windows) {
        assert_eq!(
            origin("file:///C:/notes/a.png#preview", None),
            Origin::File(PathBuf::from("C:/notes/a.png"))
        );
    }
}

#[test]
fn a_file_on_another_host_is_its_share_or_refused() {
    let found = origin("file://server/share/a.png", Some(Path::new("/notes")));
    match cfg!(windows) {
        true => assert_eq!(found, Origin::File(PathBuf::from("//server/share/a.png"))),
        false => assert!(matches!(found, Origin::Bad(_)), "{found:?}"),
    }
}

#[test]
fn unchanged_inline_addresses_reuse_bytes_and_replacements_keep_old_snapshots() {
    let mut cache = OriginCache::default();
    let data = |source| match source {
        Origin::Data(held) => held,
        _ => panic!("inline bytes"),
    };
    let first = data(cache.get("data:image/png;base64,cG5n", None));
    let repeated = data(cache.get("data:image/png;base64,cG5n", None));
    assert!(
        Arc::ptr_eq(&first.bytes, &repeated.bytes),
        "unchanged data must reuse its decoded bytes"
    );
    let next = data(cache.get("data:image/png;base64,bmV4dA==", None));
    assert_eq!(next.bytes.as_ref(), b"next");
    assert_eq!(
        first.bytes.as_ref(),
        b"png",
        "old resource snapshots remain immutable"
    );
    assert!(!Arc::ptr_eq(&first.bytes, &next.bytes));
    assert!(matches!(
        cache.get("data:image/png;base64,***", None),
        Origin::Bad(_)
    ));
    let restored = data(cache.get("DATA:image/png;base64,cG5n", None));
    assert_eq!(restored.bytes.as_ref(), b"png");
}

#[test]
fn resolved_addresses_follow_source_folder_and_file_origin_changes() {
    let mut cache = OriginCache::default();
    assert_eq!(
        cache.get("diagram.png", Some(Path::new("/first"))),
        Origin::File("/first/diagram.png".into())
    );
    assert_eq!(
        cache.get("diagram.png", Some(Path::new("/next"))),
        Origin::File("/next/diagram.png".into())
    );
    assert_eq!(cache.get("diagram.png", None), Origin::Loose);
    assert_eq!(
        cache.get("HTTPS://example.com/image.png", None),
        Origin::Web
    );
    assert!(matches!(
        cache.get("unsupported:diagram", None),
        Origin::Bad(_)
    ));
    assert_eq!(
        cache.get("file:///next/a%20b.png", None),
        Origin::File("/next/a b.png".into())
    );
}

#[test]
fn loading_inline_pictures_show_the_address_type_without_its_payload() {
    for head in ["data:image/png;base64", "DATA:image/svg+xml;base64"] {
        let source = format!("{head},{}", "a".repeat(100_000));
        let shown = pending_address(&source);
        assert_eq!(
            shown.len(),
            head.len() + ",…".len(),
            "loading text excludes payload"
        );
        assert_eq!(shown.as_ref(), format!("{head},…"));
    }
    assert_eq!(
        pending_address("file:///notes/diagram.png").as_ref(),
        "file:///notes/diagram.png"
    );
    assert_eq!(pending_address("cid:diagram").as_ref(), "cid:diagram");
}

#[test]
fn data_addresses_carry_their_bytes() {
    assert_eq!(
        origin("data:image/png;base64,cG5n", None),
        Origin::Data(HeldPicture::new(ImageFormat::Png, &b"png"[..]))
    );
    assert!(matches!(origin("data:image/png,raw", None), Origin::Bad(_)));
    assert!(matches!(
        origin("data:text/plain;base64,eA==", None),
        Origin::Bad(_)
    ));
    assert!(matches!(
        origin("data:image/png;base64,***", None),
        Origin::Bad(_)
    ));
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::RgbaImage::new(width, height)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn address(width: u32, height: u32) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png(width, height))
    )
}

struct Preview {
    url: String,
    held: Option<Result<HeldPicture, gpui::SharedString>>,
}

impl Render for Preview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(500.)).child(Picture {
            id: "sample".into(),
            alt: "fixture".into(),
            url: self.url.clone(),
            base: None,
            held: self.held.clone(),
        })
    }
}

fn expect_size(cx: &mut VisualTestContext, expected: Option<gpui::Size<gpui::Pixels>>) {
    for _ in 0..50 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        if cx
            .debug_bounds("markdown-picture")
            .map(|bounds| bounds.size)
            == expected
        {
            return;
        }
    }
    assert_eq!(
        cx.debug_bounds("markdown-picture")
            .map(|bounds| bounds.size),
        expected
    );
}

#[gpui::test]
fn rendered_pictures_replace_inline_sources_errors_and_owner_overrides(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, visual) = cx.add_window_view(|_, _| Preview {
        url: address(120, 80),
        held: None,
    });
    expect_size(visual, Some(size(px(120.), px(80.))));
    view.update(visual, |view, cx| {
        view.url = address(80, 160);
        cx.notify();
    });
    expect_size(visual, Some(size(px(80.), px(160.))));
    view.update(visual, |view, cx| {
        view.url = "data:image/png;base64,***".into();
        cx.notify();
    });
    expect_size(visual, None);
    view.update(visual, |view, cx| {
        view.held = Some(Ok(HeldPicture::new(gpui::ImageFormat::Png, png(200, 100))));
        cx.notify();
    });
    expect_size(visual, Some(size(px(200.), px(100.))));
    view.update(visual, |view, cx| {
        view.held = None;
        view.url = address(96, 64);
        cx.notify();
    });
    expect_size(visual, Some(size(px(96.), px(64.))));
    visual.update(|window, _| window.remove_window());
}

#[test]
fn resolved_inline_bytes_are_released_with_the_last_cache_and_snapshot() {
    let mut cache = OriginCache::default();
    let Origin::Data(held) = cache.get("data:image/png;base64,cG5n", None) else {
        panic!("data")
    };
    let weak = Arc::downgrade(&held.bytes);
    drop(cache);
    assert!(
        weak.upgrade().is_some(),
        "a retained frame keeps its immutable source"
    );
    drop(held);
    assert!(
        weak.upgrade().is_none(),
        "no global address cache retains the bytes"
    );
}

#[test]
#[ignore = "release address/base64 measurement; excludes pixel decoding and frame rendering"]
fn measure_picture_sources() {
    use std::{hint::black_box, time::Instant};

    for width in [8, 128, 512] {
        let mut image = image::RgbaImage::new(width, width);
        let mut seed = 0x1234_5678u32;
        for byte in image.as_mut() {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *byte = seed as u8;
        }
        let mut png = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let uri = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        let mut cache = OriginCache::default();
        let Origin::Data(held) = cache.get(&uri, None) else {
            panic!("valid PNG")
        };
        assert_eq!(held.bytes.as_ref(), png);
        for reuse in [false, true] {
            let mut samples = Vec::new();
            for _ in 0..=100 {
                let start = Instant::now();
                let resolved = if reuse {
                    cache.get(black_box(&uri), None)
                } else {
                    origin(black_box(&uri), None)
                };
                black_box(resolved);
                samples.push(start.elapsed().as_secs_f64() * 1_000_000.);
            }
            samples.remove(0);
            samples.sort_by(f64::total_cmp);
            eprintln!(
                "picture source width={width} png_bytes={} uri_bytes={} reused={reuse} trials=100 median_us={:.3}",
                png.len(),
                uri.len(),
                samples[50]
            );
        }
    }
}
