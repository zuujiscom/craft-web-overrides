use crafts_sync::http::Http;
use crafts_sync::sync::{Config, run, unpack};
use crafts_sync::{R, util};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Fake {
    files: RefCell<HashMap<String, Vec<u8>>>,
    redirects: RefCell<HashMap<String, String>>,
}
impl Http for Fake {
    fn download(&self, url: &str, dest: &Path, _t: u64, _m: u64) -> R<()> {
        let b = self.files.borrow().get(url).cloned().ok_or(format!("404 {url}"))?;
        fs::write(dest, b)?;
        Ok(())
    }
    fn redirect_location(&self, url: &str, _t: u64) -> R<Option<String>> {
        Ok(self.redirects.borrow().get(url).cloned())
    }
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (n, b) in entries {
        w.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b).unwrap();
    }
    w.finish().unwrap().into_inner()
}

fn release_zip(app: &str, ver: &str, marker: &str) -> Vec<u8> {
    let d = format!("{app}-web-{ver}");
    zip_of(&[
        (&format!("{d}/index.html"), format!("<html>{marker}</html>").as_bytes()),
        (&format!("{d}/app/{app}.wasm"), &vec![7u8; 5000]),
        (&format!("{d}/app/glue.js"), b"export const a = 1;"),
        (&format!("{d}/app/pic.png"), b"png"),
        (&format!("{d}/build/junk.o"), b"junk"),
        (&format!("{d}/deps/x"), b"junk"),
        (&format!("{d}/sub/build/keep.txt"), b"nested build dirs stay"),
    ])
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

const OV_BASE: &str = "https://github.com/zuujiscom/craft-web-overrides/releases/download";

fn scratch(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("crafts-sync-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    p
}

fn cfg(root: &Path, apps: &[&str]) -> Config {
    Config {
        root: root.into(),
        keep: 3,
        apps: apps.iter().map(|s| s.to_string()).collect(),
        manifest_url: "https://m/manifest.json".into(),
    }
}

fn official(h: &Fake, app: &str, ver: &str) {
    h.redirects.borrow_mut().insert(
        format!("https://github.com/storytold/{app}/releases/latest"),
        format!("https://github.com/storytold/{app}/releases/tag/v{ver}"),
    );
    h.files.borrow_mut().insert(
        format!("https://github.com/storytold/{app}/releases/download/v{ver}/{app}-web-{ver}.zip"),
        release_zip(app, ver, &format!("official {app} {ver}")),
    );
}

fn logs() -> (RefCell<Vec<String>>,) {
    (RefCell::new(Vec::new()),)
}

#[test]
fn full_pass_official_and_override() {
    let root = scratch("full");
    let h = Fake::default();
    official(&h, "deckcraft", "0.14.1");
    let oz = release_zip("cadcraft", "0.16.0", "patched cadcraft");
    h.files.borrow_mut().insert(format!("{OV_BASE}/cadcraft-0.16.0-zabc/cadcraft-web-0.16.0.zip"), oz.clone());
    // official cadcraft exists too, but the manifest wins
    official(&h, "cadcraft", "0.15.0");
    let manifest = format!(
        r#"{{"cadcraft":{{"dir":"0.16.0-zabc","url":"{OV_BASE}/cadcraft-0.16.0-zabc/cadcraft-web-0.16.0.zip","sha256":"{}","release":"https://rel/cad"}}}}"#,
        sha(&oz)
    );
    h.files.borrow_mut().insert("https://m/manifest.json".into(), manifest.into_bytes());
    let icon = [&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a][..], b"data"].concat();
    h.files.borrow_mut().insert("https://raw.githubusercontent.com/storytold/deckcraft/main/assets/app-icon/hicolor/128x128/apps/ai.storyteller.deckcraft.png".into(), icon);
    // cadcraft's icon is not a PNG -> letter tile, logged
    h.files.borrow_mut().insert("https://raw.githubusercontent.com/storytold/cadcraft/main/assets/app-icon/hicolor/128x128/apps/ai.storyteller.cadcraft.png".into(), b"<html>".to_vec());

    let (lg,) = logs();
    let log = |m: &str| lg.borrow_mut().push(m.to_owned());
    let now = 1_800_000_000;
    run(&cfg(&root, &["deckcraft", "cadcraft", "soundcraft"]), &h, now, &log).unwrap();

    assert_eq!(fs::read_link(root.join("www/deckcraft")).unwrap(), Path::new("../apps/deckcraft/versions/0.14.1"));
    assert_eq!(fs::read_link(root.join("www/cadcraft")).unwrap(), Path::new("../apps/cadcraft/versions/0.16.0-zabc"));
    assert_eq!(fs::read_to_string(root.join("www/cadcraft/index.html")).unwrap(), "<html>patched cadcraft</html>");
    let d = root.join("apps/deckcraft/versions/0.14.1");
    assert!(d.join(".complete").is_file());
    assert!(!d.join("build").exists() && !d.join("deps").exists());
    assert!(d.join("sub/build/keep.txt").is_file());
    // gzip and brotli copies decompress to the original
    let mut gz = Vec::new();
    std::io::copy(&mut flate2::read::GzDecoder::new(fs::File::open(d.join("app/glue.js.gz")).unwrap()), &mut gz)
        .unwrap();
    assert_eq!(gz, b"export const a = 1;");
    assert!(
        d.join("index.html.gz").is_file() && !d.join("app/pic.png.gz").exists() && !d.join("app/pic.png.br").exists()
    );
    let mut br = Vec::new();
    std::io::copy(
        &mut brotli::Decompressor::new(fs::File::open(d.join("app/deckcraft.wasm.br")).unwrap(), 4096),
        &mut br,
    )
    .unwrap();
    assert_eq!(br, vec![7u8; 5000]);
    assert!(!d.join("app/glue.js.br.tmp").exists());
    // no scratch left behind, manifest cached
    assert_eq!(fs::read_dir(root.join("tmp")).unwrap().count(), 0);
    assert!(root.join("overrides.json").is_file());
    let page = fs::read_to_string(root.join("www/index.html")).unwrap();
    assert!(page.contains("<img class=\"i\" src=\"/icons/deckcraft.png?v="));
    assert!(page.contains("<span class=\"i\" aria-hidden=\"true\">Cd</span>"));
    assert!(page.contains("<span>v0.16.0</span><em title=") && page.contains("href=\"https://rel/cad\""));
    assert!(!page.contains("href=\"/soundcraft/\""), "no link, no card");
    let l = lg.borrow().join("\n");
    assert!(
        l.contains("deckcraft: now serving 0.14.1")
            && l.contains("cadcraft: new release 0.16.0-zabc (craft-web-overrides)")
    );
    assert!(
        l.contains("soundcraft: could not read latest release, skipping")
            && l.contains("cadcraft: could not fetch its icon")
    );
    assert!(l.contains("sync finished with changes") && l.contains("brotli: deckcraft/versions/0.14.1/app/glue.js"));

    // second pass, nothing new: no download, no change message, idle log
    lg.borrow_mut().clear();
    run(&cfg(&root, &["deckcraft", "cadcraft"]), &h, now + 60, &log).unwrap();
    assert!(!lg.borrow().iter().any(|m| m.contains("new release") || m.contains("finished")), "{:?}", lg.borrow());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn bad_override_is_skipped_and_dropped_override_goes_back_to_official() {
    let root = scratch("bad");
    let h = Fake::default();
    let z = release_zip("cadcraft", "0.16.0", "patched");
    let url = format!("{OV_BASE}/cadcraft-0.16.0-zabc/cadcraft-web-0.16.0.zip");
    h.files.borrow_mut().insert(url.clone(), z.clone());
    official(&h, "cadcraft", "0.15.0");
    let man = |sum: &str, url: &str| {
        format!(r#"{{"cadcraft":{{"dir":"0.16.0-zabc","url":"{url}","sha256":"{sum}","release":"r"}}}}"#).into_bytes()
    };
    let (lg,) = logs();
    let log = |m: &str| lg.borrow_mut().push(m.to_owned());
    let c = cfg(&root, &["cadcraft"]);

    h.files.borrow_mut().insert("https://m/manifest.json".into(), man(&"0".repeat(64), &url));
    run(&c, &h, 1_800_000_000, &log).unwrap();
    assert!(lg.borrow().iter().any(|m| m.contains("doesn't match the manifest's sha256")));
    assert!(!root.join("www/cadcraft").exists(), "a failed override serves nothing new");
    assert!(fs::read_dir(root.join("tmp")).unwrap().count() == 0);

    // a url outside the project's releases is refused without a download
    h.files.borrow_mut().insert("https://m/manifest.json".into(), man(&sha(&z), "https://evil.example/x.zip"));
    run(&c, &h, 1_800_000_100, &log).unwrap();
    assert!(lg.borrow().iter().any(|m| m.contains("isn't a craft-web-overrides release")));

    // manifest unreadable: the last good one (the refused one) is kept, still nothing served
    h.files.borrow_mut().remove("https://m/manifest.json");
    run(&c, &h, 1_800_000_200, &log).unwrap();
    assert!(lg.borrow().iter().any(|m| m.contains("using the last one")));
    assert!(!root.join("www/cadcraft").exists());

    // override removed from the manifest: back to the official release
    h.files.borrow_mut().insert("https://m/manifest.json".into(), b"{}".to_vec());
    run(&c, &h, 1_800_000_300, &log).unwrap();
    assert_eq!(fs::read_link(root.join("www/cadcraft")).unwrap(), Path::new("../apps/cadcraft/versions/0.15.0"));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn keeps_three_versions_and_repairs_the_link() {
    let root = scratch("keep");
    let h = Fake::default();
    h.files.borrow_mut().insert("https://m/manifest.json".into(), b"{}".to_vec());
    let c = cfg(&root, &["deckcraft"]);
    let log = |_: &str| {};
    for (i, v) in ["0.1.0", "0.2.0", "0.3.0", "0.4.0", "0.5.0"].iter().enumerate() {
        official(&h, "deckcraft", v);
        run(&c, &h, 1_800_000_000 + i as u64, &log).unwrap();
        // distinct mtimes for the newest-first ordering
        let d = root.join(format!("apps/deckcraft/versions/{v}"));
        let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000 + i as u64 * 100);
        fs::File::open(&d).unwrap().set_modified(t).unwrap();
    }
    let mut kept: Vec<String> = fs::read_dir(root.join("apps/deckcraft/versions"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    kept.sort();
    assert_eq!(kept, ["0.3.0", "0.4.0", "0.5.0"]);
    // an interrupted swap: link points elsewhere -> repaired without a download
    fs::remove_file(root.join("www/deckcraft")).unwrap();
    std::os::unix::fs::symlink("../apps/deckcraft/versions/0.3.0", root.join("www/deckcraft")).unwrap();
    let (lg,) = logs();
    run(&c, &h, 1_800_001_000, &|m: &str| lg.borrow_mut().push(m.into())).unwrap();
    assert_eq!(fs::read_link(root.join("www/deckcraft")).unwrap(), Path::new("../apps/deckcraft/versions/0.5.0"));
    assert!(lg.borrow().iter().any(|m| m.contains("finished with changes")));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn rejects_bad_versions_and_zips() {
    let root = scratch("badzip");
    let h = Fake::default();
    h.files.borrow_mut().insert("https://m/manifest.json".into(), b"{}".to_vec());
    // a tag that isn't a plain version
    h.redirects.borrow_mut().insert(
        "https://github.com/storytold/deckcraft/releases/latest".into(),
        "https://github.com/storytold/deckcraft/releases/tag/v1..2".into(),
    );
    // a release without index.html
    h.redirects.borrow_mut().insert(
        "https://github.com/storytold/cadcraft/releases/latest".into(),
        "https://github.com/storytold/cadcraft/releases/tag/v1.0.0".into(),
    );
    h.files.borrow_mut().insert(
        "https://github.com/storytold/cadcraft/releases/download/v1.0.0/cadcraft-web-1.0.0.zip".into(),
        zip_of(&[("cadcraft-web-1.0.0/a.txt", b"x")]),
    );
    // a zip that tries to leave its folder
    h.redirects.borrow_mut().insert(
        "https://github.com/storytold/gridcraft/releases/latest".into(),
        "https://github.com/storytold/gridcraft/releases/tag/v1.0.0".into(),
    );
    h.files.borrow_mut().insert(
        "https://github.com/storytold/gridcraft/releases/download/v1.0.0/gridcraft-web-1.0.0.zip".into(),
        zip_of(&[("../evil", b"x"), ("g/index.html", b"x")]),
    );
    // not a zip at all
    h.redirects.borrow_mut().insert(
        "https://github.com/storytold/pdfcraft/releases/latest".into(),
        "https://github.com/storytold/pdfcraft/releases/tag/v1.0.0".into(),
    );
    h.files.borrow_mut().insert(
        "https://github.com/storytold/pdfcraft/releases/download/v1.0.0/pdfcraft-web-1.0.0.zip".into(),
        b"<html>".to_vec(),
    );
    let (lg,) = logs();
    run(&cfg(&root, &["deckcraft", "cadcraft", "gridcraft", "pdfcraft", "../x"]), &h, 1_800_000_000, &|m: &str| {
        lg.borrow_mut().push(m.into())
    })
    .unwrap();
    let l = lg.borrow().join("\n");
    assert!(l.contains("version '1..2' isn't a plain version name"));
    assert!(l.contains("cadcraft: no index.html in release"));
    assert!(l.contains("gridcraft: zip failed") && l.contains("pdfcraft: zip failed"));
    assert!(l.contains("../x: not a plain app name"));
    for a in ["deckcraft", "cadcraft", "gridcraft", "pdfcraft"] {
        assert!(fs::symlink_metadata(root.join("www").join(a)).is_err(), "{a}");
        assert!(
            !root.join("apps").join(a).join("versions").exists()
                || fs::read_dir(root.join("apps").join(a).join("versions")).unwrap().count() == 0,
            "{a}"
        );
    }
    assert!(!root.join("evil").exists() && !root.join("tmp/evil").exists());
    assert_eq!(fs::read_dir(root.join("tmp")).unwrap().count(), 0);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn unpack_normalises_permissions_and_refuses_links() {
    let root = scratch("unpack");
    fs::create_dir_all(&root).unwrap();
    let z = root.join("a.zip");
    fs::write(&z, zip_of(&[("d/sub/f.txt", b"hi")])).unwrap();
    unpack(&z, &root.join("out")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(fs::metadata(root.join("out/d/sub")).unwrap().permissions().mode() & 0o777, 0o755);
    assert_eq!(fs::metadata(root.join("out/d/sub/f.txt")).unwrap().permissions().mode() & 0o777, 0o644);
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    w.add_symlink("d/l", "/etc/passwd", zip::write::SimpleFileOptions::default()).unwrap();
    fs::write(&z, w.finish().unwrap().into_inner()).unwrap();
    assert!(unpack(&z, &root.join("out2")).is_err());
    fs::remove_dir_all(&root).unwrap();
    let _ = util::iso(0);
}
