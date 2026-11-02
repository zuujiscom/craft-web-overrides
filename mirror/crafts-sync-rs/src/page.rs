//! The landing page, byte for byte what the bash crafts-sync wrote.
use crate::manifest::{Override, display_version};

const HEAD: &str = include_str!("head.html");
const TAIL: &str = include_str!("tail.html");

pub struct Info {
    pub name: String,
    pub kind: &'static str,
    pub tile: String,
    pub desc: &'static str,
}

/// Name, kind, tile letters and description for each app's card, taken from the app's README (the tagline
/// under its logo and its feature list); don't add anything the README doesn't say. Text is already HTML.
pub fn app_info(app: &str) -> Info {
    let (n, k, t, d) = match app {
        "photocraft" => (
            "PhotoCraft",
            "Image editor",
            "Ph",
            "Layers, masks, adjustment layers, layer styles, type, vectors and brushes, and it opens and saves real PSD files.",
        ),
        "filmcraft" => (
            "FilmCraft",
            "Video editor",
            "Fm",
            "A non-linear video editor: edit, colour-grade, add effects, motion, titles and captions, mix the audio and export.",
        ),
        "lightcraft" => (
            "LightCraft",
            "Photo library &amp; raw developer",
            "Lt",
            "Organise your photos and develop raw files: light, colour, tone curve, colour grading, masking, presets, crop and black &amp; white. Every adjustment is non-destructive.",
        ),
        "vectorcraft" => (
            "VectorCraft",
            "Vector illustration",
            "Vc",
            "The Pen, Direct Selection, Pathfinder, Smart Guides, Appearance, Swatches and Layers. Imports and exports SVG and PDF; exports PNG, JPEG and WebP.",
        ),
        "pdfcraft" => (
            "PdfCraft",
            "PDF reader &amp; editor",
            "Pd",
            "Read, organise, combine, split and secure PDFs, with comments, forms, layers and attachments.",
        ),
        "effectcraft" => (
            "EffectCraft",
            "Motion graphics &amp; VFX",
            "Fx",
            "A compositor with compositions, layers, keyframes, 308 effects, layer styles, expressions, 3D cameras and lights, and a render queue.",
        ),
        "designcraft" => (
            "DesignCraft",
            "Page layout",
            "Dn",
            "Spreads and parent pages, threaded text frames, paragraph and character styles, tables and text wrap. Exports PDF, EPUB, PNG and IDML.",
        ),
        "wordcraft" => (
            "WordCraft",
            "Word processor",
            "Wd",
            "The ribbon, styles, tables, track changes, references and mail merge. Reads and writes .docx.",
        ),
        "gridcraft" => (
            "GridCraft",
            "Spreadsheet",
            "Gd",
            "500+ worksheet functions with dynamic arrays, tables, charts, conditional formatting, sort and AutoFilter. XLSX is its native format; CSV too.",
        ),
        "deckcraft" => (
            "DeckCraft",
            "Presentations",
            "Dk",
            "Themes, layouts, shapes, pictures, tables, charts, transitions and animations. Opens and saves .pptx and presents full screen.",
        ),
        "cadcraft" => (
            "CADCraft",
            "2D &amp; 3D CAD",
            "Cd",
            "The command line, object snaps, layers, dimensions, hatches, blocks and DXF drawings.",
        ),
        "soundcraft" => (
            "SoundCraft",
            "Audio workstation",
            "Sd",
            "Multitrack editing, a full mixer with plugins, sends and automation, MIDI, recording and bouncing.",
        ),
        _ => {
            return Info { name: app.to_owned(), kind: "", tile: app.chars().take(2).collect(), desc: "" };
        }
    };
    Info { name: n.to_owned(), kind: k, tile: t.to_owned(), desc: d }
}

/// Crash recovery we've checked: EffectCraft and FilmCraft have their own (our patches fix it on Safari); the
/// others only get it from our patches (craft-web-recovery). LightCraft isn't labelled until tested.
fn recovers(app: &str, patched: bool) -> bool {
    match app {
        "effectcraft" | "filmcraft" => true,
        "photocraft" | "vectorcraft" | "wordcraft" | "deckcraft" | "designcraft" | "cadcraft" | "gridcraft"
        | "pdfcraft" | "soundcraft" => patched,
        _ => false,
    }
}

/// One served app: `version` is the folder name `www/<app>` points at.
pub struct Served<'a> {
    pub app: &'a str,
    pub version: &'a str,
    /// the app's manifest entry, if it has one
    pub over: Option<&'a Override>,
    /// mtime (epoch seconds) of its icon, if one was fetched
    pub icon_mtime: Option<u64>,
}

pub fn render(apps: &[Served]) -> String {
    let mut out = String::from(HEAD);
    for s in apps {
        let info = app_info(s.app);
        let patched = s.over.filter(|o| o.dir == s.version);
        let (extra, shown) = match patched {
            Some(o) => (
                format!(
                    "<em title=\"Carries fixes not in a storytold release yet\">patched</em><a class=\"w\" href=\"{}\">What&#39;s patched</a>",
                    o.release
                ),
                display_version(s.version),
            ),
            None => (String::new(), s.version),
        };
        let rec = if recovers(s.app, patched.is_some()) {
            "<p class=\"r ok\">&#10003; Autosave: recovers unsaved work</p>"
        } else {
            "<p class=\"r\"></p>"
        };
        let tile = match s.icon_mtime {
            Some(m) => {
                format!("<img class=\"i\" src=\"/icons/{}.png?v={m}\" alt=\"\" width=\"52\" height=\"52\">", s.app)
            }
            None => format!("<span class=\"i\" aria-hidden=\"true\">{}</span>", info.tile),
        };
        out.push_str(&format!(
            "<li class=\"a-{app}\"><div class=\"h\">{tile}<div><h2><a href=\"/{app}/\">{name}</a></h2><span class=\"k\">{kind}</span></div></div><p class=\"d\">{desc}</p>{rec}<div class=\"f\"><span>v{shown}</span>{extra}<span class=\"go\" aria-hidden=\"true\">Open &rarr;</span></div></li>\n",
            app = s.app,
            name = info.name,
            kind = info.kind,
            desc = info.desc,
        ));
    }
    out.push_str(TAIL);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ov(dir: &str) -> Override {
        Override { dir: dir.into(), url: "u".into(), sha256: "s".into(), release: "https://r/tag".into() }
    }

    #[test]
    fn head_and_tail() {
        let p = render(&[]);
        assert!(p.starts_with("<!doctype html><html lang=\"en\">"));
        assert!(p.contains("\n<ul>\n</ul><footer>Unofficial web hosting"));
        assert!(p.ends_with("</main></body></html>\n"));
    }

    #[test]
    fn plain_card_with_icon() {
        let p = render(&[Served { app: "deckcraft", version: "0.14.1", over: None, icon_mtime: Some(1_700_000_000) }]);
        let li = p.lines().find(|l| l.starts_with("<li")).unwrap();
        assert_eq!(
            li,
            "<li class=\"a-deckcraft\"><div class=\"h\"><img class=\"i\" src=\"/icons/deckcraft.png?v=1700000000\" alt=\"\" width=\"52\" height=\"52\"><div><h2><a href=\"/deckcraft/\">DeckCraft</a></h2><span class=\"k\">Presentations</span></div></div><p class=\"d\">Themes, layouts, shapes, pictures, tables, charts, transitions and animations. Opens and saves .pptx and presents full screen.</p><p class=\"r\"></p><div class=\"f\"><span>v0.14.1</span><span class=\"go\" aria-hidden=\"true\">Open &rarr;</span></div></li>"
        );
    }

    #[test]
    fn patched_card_with_tile_and_autosave() {
        let o = ov("0.11.1-zb4b5734d");
        let p = render(&[Served { app: "photocraft", version: "0.11.1-zb4b5734d", over: Some(&o), icon_mtime: None }]);
        let li = p.lines().find(|l| l.starts_with("<li")).unwrap();
        assert!(li.contains("<span class=\"i\" aria-hidden=\"true\">Ph</span>"));
        assert!(li.contains("<p class=\"r ok\">&#10003; Autosave: recovers unsaved work</p>"));
        assert!(li.contains("<span>v0.11.1</span><em title=\"Carries fixes not in a storytold release yet\">patched</em><a class=\"w\" href=\"https://r/tag\">What&#39;s patched</a><span class=\"go\""));
    }

    #[test]
    fn override_not_served_is_not_a_badge() {
        // the manifest names another folder than the one served (official release serving): no badge, no autosave for photocraft
        let o = ov("0.11.1-zb4b5734d");
        let p = render(&[Served { app: "photocraft", version: "0.12.0", over: Some(&o), icon_mtime: None }]);
        assert!(!p.contains("patched</em>") && !p.contains("class=\"r ok\""));
        // effectcraft has its own recovery either way
        let p = render(&[Served { app: "effectcraft", version: "0.19.0", over: None, icon_mtime: None }]);
        assert!(p.contains("class=\"r ok\""));
    }

    #[test]
    fn unknown_app_gets_plain_card() {
        let p = render(&[Served { app: "xyzcraft", version: "1", over: None, icon_mtime: None }]);
        assert!(p.contains("<h2><a href=\"/xyzcraft/\">xyzcraft</a></h2><span class=\"k\"></span>"));
        assert!(p.contains("<span class=\"i\" aria-hidden=\"true\">xy</span>"));
    }
}
