#!/usr/bin/env python3
"""Publishes the builds of one workflow run and rewrites manifest.json and STATUS.md.

    tools/publish.py <results dir>     (each app's prepare result.json and zip in <results dir>/result-<app>/)

For an app built in this run: a release `<app>-<version>-z<id>` with the zip, and its manifest entry.
An app whose patches are all dropped (`status: upstream`) or that left overrides.json leaves the manifest, so
the mirror goes back to storytold's own zip. So does an app whose prepare or build failed: the newest release
as storytold shipped it beats an old patched build.
Needs gh with GH_TOKEN (contents: write) and GITHUB_REPOSITORY.
"""
import datetime, glob, hashlib, json, os, re, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
REPO = os.environ.get("GITHUB_REPOSITORY", "zuujiscom/craft-web-overrides")
# Same as prepare.py's; the version and id end up in a release name and a directory name on the mirror.
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?")
ID = re.compile(r"[0-9a-f]{8}")


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def notes(r):
    lines = [f"{r['app']} {r['version']} (storytold {r['tag']}) with these patches:", ""]
    for p in r["applied"]:
        lines.append(f"- {p['patch']}: {p['why']} ({', '.join(c[:10] for c in p['commits'])})")
    for u in r.get("libs", []):
        lines.append(f"- library {u['branch']} at {u['commit'][:10]} ({', '.join(u['packages'])}): {u['why']}")
    for d in r["dropped"]:
        lines.append(f"- dropped {d['patch']}: {d['reason']}")
    return "\n".join(lines)


def main():
    results_dir = sys.argv[1]
    overrides = json.load(open(os.path.join(ROOT, "overrides.json")))["apps"]
    path = os.path.join(ROOT, "manifest.json")
    manifest = json.load(open(path)) if os.path.exists(path) else {}
    status = []
    for app in sorted(overrides):
        files = glob.glob(os.path.join(results_dir, f"result-{app}", "result.json"))
        if not files:
            manifest.pop(app, None)
            status.append((app, "failed: no result (see the run log); serving storytold's release"))
            continue
        r = json.load(open(files[0]))
        # The build job that uploaded result.json also ran the app's own build scripts, so check
        # what it says before it names a release or a directory.
        if r.get("app") != app or not VERSION.fullmatch(str(r.get("version"))) or ".." in str(r.get("version")) \
                or not ID.fullmatch(str(r.get("id"))):
            manifest.pop(app, None)
            status.append((app, "failed: result.json doesn't match what prepare.py writes; serving storytold's release"))
            continue
        if r["status"] == "upstream":
            manifest.pop(app, None)
            status.append((app, f"storytold {r['tag']} as released: " + "; ".join(f"{d['patch']} {d['reason']}" for d in r["dropped"])))
            continue
        if not r["build"]:
            status.append((app, f"unchanged: {manifest[app]['dir']}"))
            continue
        zips = glob.glob(os.path.join(results_dir, f"result-{app}", f"{app}-web-*.zip"))
        if len(zips) != 1:
            manifest.pop(app, None)
            status.append((app, f"failed: build gave {len(zips)} zips; serving storytold's release"))
            continue
        tag = f"{app}-{r['version']}-z{r['id']}"
        name = os.path.basename(zips[0])
        subprocess.run(["gh", "release", "create", tag, zips[0], "-R", REPO, "--title", f"{app} {r['version']} + {len(r['applied'])} patch(es)",
                        "--notes", notes(r), "--target", os.environ.get("GITHUB_SHA", "main")], check=True)
        manifest[app] = {
            "dir": f"{r['version']}-z{r['id']}", "version": r["version"], "id": r["id"], "upstream_tag": r["tag"],
            "url": f"https://github.com/{REPO}/releases/download/{tag}/{name}", "sha256": sha256(zips[0]),
            "release": f"https://github.com/{REPO}/releases/tag/{tag}", "source": r["head"],
            "patches": [{"patch": p["patch"], "why": p["why"]} for p in r["applied"]]
            + [{"patch": f"{u['branch']}@{u['commit'][:10]}", "why": u["why"]} for u in r.get("libs", [])],
        }
        status.append((app, f"built {tag}"))
    for app in [a for a in manifest if a not in overrides]:
        manifest.pop(app)
        status.append((app, "removed from overrides.json: back to storytold's release"))
    json.dump(manifest, open(path, "w"), indent=2, sort_keys=True)
    open(path, "a").write("\n")
    now = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    with open(os.path.join(ROOT, "STATUS.md"), "w") as f:
        f.write(f"# Status\n\nLast run {now}.\n\n| App | Result |\n| --- | --- |\n")
        for app, text in status:
            f.write(f"| {app} | {text} |\n")
        f.write("\n## Served\n\n")
        for app, e in sorted(manifest.items()):
            f.write(f"- **{app}** {e['dir']} ([release]({e['release']})): " + "; ".join(p["patch"] for p in e["patches"]) + "\n")
        if not manifest:
            f.write("Nothing: every app is storytold's own release.\n")
    print("\n".join(f"{a}: {t}" for a, t in status))


if __name__ == "__main__":
    main()
