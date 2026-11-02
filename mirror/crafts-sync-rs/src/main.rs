#![forbid(unsafe_code)]
//! Mirrors the browser builds of the storytold/ArtCraft apps into CRAFTS_ROOT (default /srv/crafts): for each
//! app, the latest GitHub release's `<app>-web-<ver>.zip` (release redirect, no API, so no rate limit) or, if
//! craft-web-overrides' manifest.json lists the app, that repo's patched build (checked against the manifest's
//! sha256). Unpacked into versions/<ver>, public link flipped atomically, the last 3 versions kept, gzip and
//! brotli copies made, icons fetched daily, landing page regenerated.
//! Source: https://github.com/zuujiscom/craft-web-overrides (mirror/crafts-sync-rs).
use crafts_sync::{http::Ureq, sync, util::iso};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn main() -> ExitCode {
    let cfg = sync::Config::from_env(|k| std::env::var(k).ok());
    let log = |m: &str| println!("{} {m}", iso(now()));
    match sync::run(&cfg, &Ureq, now(), &log) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("crafts-sync: {e}");
            ExitCode::FAILURE
        }
    }
}
