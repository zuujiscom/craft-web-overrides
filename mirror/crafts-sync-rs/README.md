# crafts-sync (Rust)

Rust replacement for `mirror/crafts-sync` (bash + python3 + curl + unzip + gzip + brotli). Same behaviour, one
static-ish binary, no other tools on the box. Run every 15 minutes by `systemd/crafts-sync.timer` as user `crafts`.

Environment (same as the bash script): `CRAFTS_ROOT` (default `/srv/crafts`), `CRAFTS_APPS` (space separated),
`CRAFTS_MANIFEST_URL`.

What a run does, in order:

1. Creates `apps/ www/ tmp/` under the root, takes `.lock` (non-blocking; "another sync is running" and exit 0).
2. Fetches `manifest.json` from craft-web-overrides (30 s). Valid JSON replaces `overrides.json`; otherwise logs
   "could not read the overrides manifest; using the last one" and keeps the old file.
3. For each app: an app in the manifest is served from its `dir`/`url` (sha256 checked); otherwise the newest
   `storytold/<app>` release is read from the `releases/latest` redirect (no API) and `<app>-web-<ver>.zip` used.
   The version must be a plain name (it names a directory that is replaced). If `versions/<ver>/.complete`
   exists only the `www/<app>` link is repaired. Otherwise: download (900 s), sha256, unpack (CRCs checked),
   require `index.html`, drop Rust build junk (`.fingerprint build deps incremental`), write `.gz` (level 9) for
   wasm/js/html/css/otf/ttf/json/svg, move into `versions/<ver>`, touch `.complete`, switch the link atomically
   (new link + rename), keep the newest 3 versions.
4. Writes `.br` (quality 11) beside every wasm/js/otf/ttf under `apps/` that has none.
5. Fetches each app's icon into `www/icons/<app>.png` when the old one is over a day old (PNG magic checked).
6. Regenerates `www/index.html` (written to `.index.new`, then renamed).

Differences from the bash version (all stricter): override zip URLs must be craft-web-overrides release URLs;
https only; zip entries that leave the folder, links, and oversized downloads/unpacked trees are refused; files
are written 0644/0755; the version just made current is never pruned; an unusable manifest entry is skipped
instead of dropping every override. A failed override (download, sha256, bad zip) is skipped like in bash, so the
previous version keeps being served; it does not fall back to the official release (a manifest without the app does).

Build (on Linux):

```
cargo test
cargo build --release --locked --target aarch64-unknown-linux-gnu
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
  cargo build --release --locked --target x86_64-unknown-linux-gnu
cargo audit && cargo deny check
```
