# craft-web-overrides

Patched web builds of the [storytold](https://github.com/storytold) Craft apps, for fixes that aren't in a
storytold release yet. [craft.zuujis.com](https://craft.zuujis.com) serves storytold's own release zip for every
app, except the apps listed in [`manifest.json`](manifest.json), which it serves from this repo's releases.

## How it works

1. [`overrides.json`](overrides.json) lists, under `apps`, the patches to apply per app. A patch is an upstream
   pull request (`"pr": 977`) or a branch anywhere on GitHub (`"branch": "owner/repo:branch"`); its commits are the
   ones between storytold's `main` and the patch head. `why` says what it fixes.

   Under `libs` are **library overrides**: a branch of a shared library (our
   [craft-libs](https://github.com/zuujiscom/craft-libs) fork) that app patches depend on with
   `git = "https://github.com/zuujiscom/craft-libs", branch = "…"`. Every build moves those dependencies to the
   branch's newest commit, and that commit is part of the build id, so a fix in the library rebuilds every app
   that uses it. Code several apps need (the web builds' crash-recovery autosave, `craft-web-recovery`) lives
   there once; each app's patch only hooks it up.
2. [`.github/workflows/build.yml`](.github/workflows/build.yml) runs on every change here and every 3 hours.
   For each app, [`tools/prepare.py`](tools/prepare.py) checks out the newest storytold release tag and
   cherry-picks the patches onto it:
   - a patch whose PR was merged before that release, or whose commits are already in it, is dropped;
   - when every patch of an app is dropped, the app leaves the manifest and the mirror goes back to
     storytold's own zip;
   - a patch that no longer applies fails that app's build, and the mirror serves storytold's own release zip until the patch is fixed.
   - an app whose patched `Cargo.lock` uses a library override gets that library's newest commit.
3. The app is built the way storytold's release workflow builds it (`packaging/web/package.sh`, stable Rust,
   the same Trunk), only when its release tag or patch commits changed.
4. [`tools/publish.py`](tools/publish.py) publishes the zip as a release `<app>-<version>-z<id>`, records its
   URL and SHA-256 in `manifest.json`, and writes [`STATUS.md`](STATUS.md).
5. [`mirror/crafts-sync`](mirror/crafts-sync) (on the server, every 15 minutes) reads `manifest.json`,
   downloads each listed zip, checks its SHA-256 and serves it as version `<version>-z<id>`.

## Adding a fix

Add the PR or branch to the app's `patches` in `overrides.json` and push. For a fix of our own, push a branch
to the app's fork (`zuujiscom/<app>`), based on storytold's `main` so it can also go upstream as a PR, and list
it as a `branch` patch.

To try a patch set locally: `tools/prepare.py <app> --src /tmp/<app> --out /tmp/result --manifest /dev/null`.

## License

The tools here are MIT. The apps and the patches keep their own licenses (MIT or Apache-2.0, see each app).
