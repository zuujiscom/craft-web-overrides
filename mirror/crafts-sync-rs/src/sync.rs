use crate::R;
use crate::http::Http;
use crate::manifest::{self, Override};
use crate::page::{self, Served};
use crate::util::{files_below, mtime_secs, remove_any, sha256_hex};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

pub const DEFAULT_APPS: &str = "photocraft filmcraft lightcraft vectorcraft pdfcraft effectcraft designcraft wordcraft gridcraft deckcraft cadcraft soundcraft";
pub const DEFAULT_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/zuujiscom/craft-web-overrides/main/manifest.json";
const OWNER: &str = "storytold";
/// Not in the bash version: bounds on what one release may make us write.
const MAX_ZIP: u64 = 2 << 30;
const MAX_UNPACKED: u64 = 6 << 30;
const MAX_MANIFEST: u64 = 4 << 20;
const MAX_ICON: u64 = 4 << 20;

pub struct Config {
    pub root: PathBuf,
    pub keep: usize,
    pub apps: Vec<String>,
    pub manifest_url: String,
}

impl Config {
    /// CRAFTS_ROOT, CRAFTS_APPS (space separated) and CRAFTS_MANIFEST_URL, as in the bash script.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Config {
        let nonempty = |k: &str| get(k).filter(|v| !v.is_empty());
        Config {
            root: nonempty("CRAFTS_ROOT").unwrap_or_else(|| "/srv/crafts".into()).into(),
            keep: 3,
            apps: nonempty("CRAFTS_APPS")
                .unwrap_or_else(|| DEFAULT_APPS.into())
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
            manifest_url: nonempty("CRAFTS_MANIFEST_URL").unwrap_or_else(|| DEFAULT_MANIFEST_URL.into()),
        }
    }
}

/// One sync pass. `now` is epoch seconds. Per-app problems are logged and skipped; only a root that
/// can't be set up is an error.
pub fn run(cfg: &Config, http: &dyn Http, now: u64, log: &dyn Fn(&str)) -> R<()> {
    let root = &cfg.root;
    for d in ["apps", "www", "tmp"] {
        fs::create_dir_all(root.join(d))?;
    }
    let lock = OpenOptions::new().create(true).write(true).truncate(false).open(root.join(".lock"))?;
    if lock.try_lock().is_err() {
        println!("another sync is running");
        return Ok(());
    }

    // the overrides manifest; the last good one is kept
    let tmp_manifest = root.join("tmp/manifest.json");
    let fetched = http.download(&cfg.manifest_url, &tmp_manifest, 30, MAX_MANIFEST).is_ok()
        && fs::read(&tmp_manifest).is_ok_and(|b| manifest::is_json(&b));
    if fetched {
        fs::rename(&tmp_manifest, root.join("overrides.json"))?;
    } else {
        remove_any(&tmp_manifest);
        log("could not read the overrides manifest; using the last one");
    }
    let overrides = fs::read(root.join("overrides.json")).map(|b| manifest::parse_overrides(&b)).unwrap_or_default();

    let mut changed = false;
    for app in &cfg.apps {
        if !manifest::is_plain_app(app) {
            log(&format!("{app}: not a plain app name, skipping"));
            continue;
        }
        match sync_app(cfg, http, app, overrides.get(app), log) {
            Ok(c) => changed |= c,
            Err(e) => log(&format!("{app}: {e}")),
        }
    }

    brotli_pass(root, log);
    icons(cfg, http, now, log);
    if let Err(e) = write_page(cfg, &overrides) {
        log(&format!("landing page: {e}"));
    }
    if changed {
        log("sync finished with changes");
    }
    Ok(())
}

fn link_target(app: &str, ver: &str) -> String {
    format!("../apps/{app}/versions/{ver}")
}

/// Atomic: a new link beside the old one, then rename over it.
fn switch_link(root: &Path, app: &str, ver: &str) -> R<()> {
    let new = root.join("www").join(format!(".{app}.new"));
    remove_any(&new);
    symlink(link_target(app, ver), &new)?;
    fs::rename(&new, root.join("www").join(app))?;
    Ok(())
}

/// Returns whether the public link changed.
fn sync_app(cfg: &Config, http: &dyn Http, app: &str, over: Option<&Override>, log: &dyn Fn(&str)) -> R<bool> {
    let root = &cfg.root;
    let (tag, ver, url, sum): (String, String, String, Option<&str>) = match over {
        Some(o) => {
            if !manifest::is_trusted_override_url(&o.url) {
                log(&format!("{app}: override url {} isn't a craft-web-overrides release, skipping", o.url));
                return Ok(false);
            }
            (format!("{} (craft-web-overrides)", o.dir), o.dir.clone(), o.url.clone(), Some(&o.sha256))
        }
        None => {
            let latest = format!("https://github.com/{OWNER}/{app}/releases/latest");
            let tag = match http.redirect_location(&latest, 30) {
                Ok(Some(loc)) => manifest::tag_from_location(&loc).to_owned(),
                _ => String::new(),
            };
            if tag.is_empty() {
                log(&format!("{app}: could not read latest release, skipping"));
                return Ok(false);
            }
            let ver = tag.strip_prefix('v').unwrap_or(&tag).to_owned();
            let url = format!("https://github.com/{OWNER}/{app}/releases/download/{tag}/{app}-web-{ver}.zip");
            (tag, ver, url, None)
        }
    };
    if !manifest::is_plain_version(&ver) {
        log(&format!("{app}: version '{ver}' isn't a plain version name, skipping"));
        return Ok(false);
    }
    let dest = root.join("apps").join(app).join("versions").join(&ver);
    if dest.join(".complete").is_file() {
        // make sure the public link points at it (first run, or a previous swap was interrupted)
        let www = root.join("www").join(app);
        if fs::read_link(&www).ok().as_deref() != Some(Path::new(&link_target(app, &ver))) {
            switch_link(root, app, &ver)?;
            return Ok(true);
        }
        return Ok(false);
    }
    log(&format!("{app}: new release {tag}"));
    let work = Work::new(&root.join("tmp"), app)?;
    let zip = work.path.join(format!("{app}-web-{ver}.zip"));
    if http.download(&url, &zip, 900, MAX_ZIP).is_err() {
        log(&format!("{app}: download failed"));
        return Ok(false);
    }
    if let Some(sum) = sum
        && !sum.eq_ignore_ascii_case(&sha256_hex(&zip)?)
    {
        log(&format!("{app}: {url} doesn't match the manifest's sha256, skipping"));
        return Ok(false);
    }
    let x = work.path.join("x");
    if let Err(e) = unpack(&zip, &x) {
        log(&format!("{app}: zip failed ({e})"));
        return Ok(false);
    }
    // the zip holds one top-level folder named <app>-web-<ver>
    let Some(inner) = top_dir_with_index(&x) else {
        log(&format!("{app}: no index.html in release, skipping"));
        return Ok(false);
    };
    // drop Rust build output some releases ship by mistake (LightCraft 0.6.0 had ~200 MB of it)
    for junk in [".fingerprint", "build", "deps", "incremental"] {
        remove_any(&inner.join(junk));
    }
    gzip_all(&inner);
    fs::create_dir_all(root.join("apps").join(app).join("versions"))?;
    remove_any(&dest);
    fs::rename(&inner, &dest)?;
    File::create(dest.join(".complete"))?;
    switch_link(root, app, &ver)?;
    drop(work);
    prune(&root.join("apps").join(app).join("versions"), cfg.keep, &ver);
    log(&format!("{app}: now serving {ver}"));
    Ok(true)
}

/// A scratch dir under tmp/ that is removed when done.
struct Work {
    path: PathBuf,
}
impl Work {
    fn new(tmp: &Path, app: &str) -> R<Work> {
        for i in 0..100u32 {
            let p = tmp.join(format!("{app}.{}-{i}", std::process::id()));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::set_permissions(&p, fs::Permissions::from_mode(0o700))?;
                    return Ok(Work { path: p });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => remove_any(&p),
                Err(e) => return Err(e.into()),
            }
        }
        Err("no scratch dir".into())
    }
}
impl Drop for Work {
    fn drop(&mut self) {
        remove_any(&self.path);
    }
}

/// Checks every entry's CRC while extracting. Refuses paths that leave `out`, and links.
pub fn unpack(zip: &Path, out: &Path) -> R<()> {
    let mut ar = zip::ZipArchive::new(File::open(zip)?)?;
    fs::create_dir_all(out)?;
    let mut total = 0u64;
    for i in 0..ar.len() {
        let mut e = ar.by_index(i)?;
        let Some(rel) = e.enclosed_name() else {
            return Err(format!("unsafe path {:?}", e.name()).into());
        };
        if e.is_symlink() {
            return Err(format!("{:?} is a link", e.name()).into());
        }
        let target = out.join(rel);
        if e.is_dir() {
            fs::create_dir_all(&target)?;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o755))?;
            continue;
        }
        if let Some(p) = target.parent() {
            fs::create_dir_all(p)?;
        }
        let mut f = File::create(&target)?;
        let n = std::io::copy(&mut (&mut e).take(MAX_UNPACKED - total + 1), &mut f)?;
        total += n;
        if total > MAX_UNPACKED {
            return Err("unpacks to more than the limit".into());
        }
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644))?;
    }
    // directories made only implicitly
    for d in dirs_below(out) {
        fs::set_permissions(d, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn dirs_below(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                out.push(e.path());
                stack.push(e.path());
            }
        }
    }
    out
}

/// The first top-level folder (by name) that has an index.html.
fn top_dir_with_index(x: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> =
        fs::read_dir(x).ok()?.flatten().filter(|e| e.file_type().is_ok_and(|t| t.is_dir())).map(|e| e.path()).collect();
    dirs.sort();
    dirs.into_iter().find(|d| d.join("index.html").is_file())
}

fn ext(p: &Path) -> &str {
    p.extension().and_then(|e| e.to_str()).unwrap_or("")
}

fn with_suffix(p: &Path, s: &str) -> PathBuf {
    let mut o = p.as_os_str().to_owned();
    o.push(s);
    o.into()
}

/// gzip -9 -n copies of text and wasm files, kept beside the originals (existing .gz are left alone).
fn gzip_all(dir: &Path) {
    for f in files_below(dir) {
        if !matches!(ext(&f), "wasm" | "js" | "html" | "css" | "otf" | "ttf" | "json" | "svg") {
            continue;
        }
        let gz = with_suffix(&f, ".gz");
        if gz.exists() {
            continue;
        }
        let res = (|| -> R<()> {
            let mut enc = flate2::GzBuilder::new().write(File::create(&gz)?, flate2::Compression::best());
            std::io::copy(&mut File::open(&f)?, &mut enc)?;
            enc.finish()?.flush()?;
            fs::set_permissions(&gz, fs::Permissions::from_mode(0o644))?;
            Ok(())
        })();
        if res.is_err() {
            remove_any(&gz);
        }
    }
}

/// Keeps the newest `keep` version folders (by mtime, then name) plus the one just made current.
fn prune(versions: &Path, keep: usize, current: &str) {
    let mut dirs: Vec<(u64, String)> = fs::read_dir(versions)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()) && !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| (mtime_secs(&e.path()).unwrap_or(0), e.file_name().to_string_lossy().into_owned()))
        .collect();
    dirs.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    for (_, name) in dirs.into_iter().skip(keep) {
        if name != current {
            remove_any(&versions.join(name));
        }
    }
}

/// .br copies (quality 11) of the big files for browsers that accept br. Covers new releases and backfills
/// older folders; existing .br files (some releases ship them) are kept.
fn brotli_pass(root: &Path, log: &dyn Fn(&str)) {
    let apps = root.join("apps");
    for f in files_below(&apps) {
        if !matches!(ext(&f), "wasm" | "js" | "otf" | "ttf") {
            continue;
        }
        let br = with_suffix(&f, ".br");
        if br.exists() {
            continue;
        }
        let tmp = with_suffix(&f, ".br.tmp");
        let res = (|| -> R<()> {
            let mut w = brotli::CompressorWriter::new(File::create(&tmp)?, 64 * 1024, 11, 22);
            std::io::copy(&mut File::open(&f)?, &mut w)?;
            w.flush()?;
            drop(w.into_inner());
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644))?;
            fs::rename(&tmp, &br)?;
            Ok(())
        })();
        match res {
            Ok(()) => log(&format!("brotli: {}", f.strip_prefix(&apps).unwrap_or(&f).display())),
            Err(e) => {
                remove_any(&tmp);
                log(&format!("brotli: {}: {e}", f.display()));
            }
        }
    }
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// App icons for the landing page, from each app's repo (refreshed daily; a failed fetch keeps the old one).
fn icons(cfg: &Config, http: &dyn Http, now: u64, log: &dyn Fn(&str)) {
    let root = &cfg.root;
    let dir = root.join("www/icons");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    for app in cfg.apps.iter().filter(|a| manifest::is_plain_app(a)) {
        let icon = dir.join(format!("{app}.png"));
        if mtime_secs(&icon).is_some_and(|m| now.saturating_sub(m) < 86400) {
            continue;
        }
        let tmp = root.join("tmp").join(format!("{app}.png"));
        let url = format!(
            "https://raw.githubusercontent.com/{OWNER}/{app}/main/assets/app-icon/hicolor/128x128/apps/ai.storyteller.{app}.png"
        );
        let ok = http.download(&url, &tmp, 30, MAX_ICON).is_ok() && {
            let mut magic = [0u8; 8];
            File::open(&tmp).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && magic == PNG_MAGIC
        };
        if ok {
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o644));
            if fs::rename(&tmp, &icon).is_err() {
                remove_any(&tmp);
            }
        } else {
            remove_any(&tmp);
            log(&format!("{app}: could not fetch its icon"));
        }
    }
}

fn write_page(cfg: &Config, overrides: &BTreeMap<String, Override>) -> R<()> {
    let www = cfg.root.join("www");
    let mut versions = Vec::new();
    for app in &cfg.apps {
        if !manifest::is_plain_app(app) {
            continue;
        }
        let Ok(m) = fs::symlink_metadata(www.join(app)) else {
            continue;
        };
        if !m.file_type().is_symlink() {
            continue;
        }
        let Ok(t) = fs::read_link(www.join(app)) else {
            continue;
        };
        let Some(v) = t.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        versions.push((app.as_str(), v.to_owned(), mtime_secs(&www.join(format!("icons/{app}.png")))));
    }
    let served: Vec<Served> = versions
        .iter()
        .map(|(app, v, icon_mtime)| Served { app, version: v, over: overrides.get(*app), icon_mtime: *icon_mtime })
        .collect();
    let new = www.join(".index.new");
    fs::write(&new, page::render(&served))?;
    fs::set_permissions(&new, fs::Permissions::from_mode(0o644))?;
    fs::rename(&new, www.join("index.html"))?;
    Ok(())
}
