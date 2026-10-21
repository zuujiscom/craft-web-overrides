#!/usr/bin/env python3
"""Checks out an app's newest storytold release and applies its patches from overrides.json.

    tools/prepare.py <app> --src <checkout dir> --out <result dir> [--manifest manifest.json] [--force]

Writes <result dir>/result.json and prints `build=true|false` for $GITHUB_OUTPUT:
- each patch is an upstream PR (`pr`), a branch (`branch`: "owner/repo:branch") or both; its commits are the
  non-merge commits between storytold's main and the patch head, cherry-picked in order;
- a patch whose PR merged before the release, or whose commits are all already in the release (same patch id),
  is dropped; when every patch is dropped the app needs no override (`status: upstream`);
- a library override (`libs`: "owner/repo:branch") is used by an app whose patches depend on that branch (its
  Cargo.lock has `git+https://github.com/<owner>/<repo>?branch=<branch>#<commit>`): its lock entries are moved
  to the branch's newest commit in the checkout, so a library fix rebuilds every app that uses it without touching
  the app patches (the web build isn't `--locked`, so cargo itself adds a dependency the library gained);
- `id` hashes the release tag, the applied commits and the library commits, so the same inputs never build twice:
  `build=false` when the manifest already has that id (unless --force).
A patch that doesn't apply fails the run; the mirror then serves storytold's release without patches.
Needs git, and GH_TOKEN for the GitHub API (only public data is read).
"""
import argparse, hashlib, json, os, re, subprocess, sys, urllib.request

UPSTREAM = "storytold"
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def api(path):
    req = urllib.request.Request(f"https://api.github.com/{path}", headers={"Accept": "application/vnd.github+json"})
    if os.environ.get("GH_TOKEN"):
        req.add_header("Authorization", f"Bearer {os.environ['GH_TOKEN']}")
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def git(src, *args, check=True):
    r = subprocess.run(["git", "-C", src, *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        sys.exit(f"git {' '.join(args)} failed:\n{r.stdout}{r.stderr}")
    return r


def is_ancestor(src, commit, tag):
    return git(src, "merge-base", "--is-ancestor", commit, tag, check=False).returncode == 0


def patch_commits(src, app, patch):
    """The patch's commits (oldest first), and a label for it."""
    if "branch" in patch:
        repo, _, branch = patch["branch"].partition(":")
        git(src, "fetch", "-q", f"https://github.com/{repo}.git", branch)
        label = patch["branch"]
    else:
        git(src, "fetch", "-q", "origin", f"pull/{patch['pr']}/head")
        label = f"{UPSTREAM}/{app}#{patch['pr']}"
    head = git(src, "rev-parse", "FETCH_HEAD").stdout.strip()
    base = git(src, "merge-base", head, "origin/main").stdout.strip()
    commits = git(src, "rev-list", "--reverse", "--no-merges", f"{base}..{head}").stdout.split()
    return label, commits


def lib_uses(src, libs):
    """Point this checkout's Cargo.lock at each library override's newest commit; the ones used:
    [{lib, branch, why, commit, packages}]."""
    lock_path = os.path.join(src, "Cargo.lock")
    lock = open(lock_path).read() if os.path.exists(lock_path) else ""
    uses = []
    for name, lib in sorted(libs.items()):
        repo, _, branch = lib["branch"].partition(":")
        source = f"git+https://github.com/{repo}?branch={branch}#"
        packages, current = [], None
        for line in lock.splitlines():
            if line.startswith("name = "):
                current = line.split('"')[1]
            elif line.startswith(f'source = "{source}') and current:
                packages.append(current)
        if not packages:
            continue
        r = subprocess.run(["git", "ls-remote", f"https://github.com/{repo}.git", f"refs/heads/{branch}"], capture_output=True, text=True)
        commit = r.stdout.split()[0] if r.returncode == 0 and r.stdout.split() else None
        if not commit:
            sys.exit(f"library override {lib['branch']}: branch not found\n{r.stderr}")
        lock = re.sub(re.escape(source) + r"[0-9a-f]{40}", source + commit, lock)
        uses.append({"lib": name, "branch": lib["branch"], "why": lib.get("why", ""), "commit": commit, "packages": sorted(set(packages))})
    if uses:
        open(lock_path, "w").write(lock)
    return uses


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("app")
    ap.add_argument("--src", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--manifest", default=os.path.join(ROOT, "manifest.json"))
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args()

    overrides = json.load(open(os.path.join(ROOT, "overrides.json")))
    patches = overrides["apps"][a.app]["patches"]
    manifest = json.load(open(a.manifest)) if os.path.exists(a.manifest) else {}
    tag = api(f"repos/{UPSTREAM}/{a.app}/releases/latest")["tag_name"]

    if not os.path.isdir(os.path.join(a.src, ".git")):
        subprocess.run(["git", "clone", "-q", "--filter=blob:none", f"https://github.com/{UPSTREAM}/{a.app}.git", a.src], check=True)
    git(a.src, "fetch", "-q", "--tags", "origin", "main")
    git(a.src, "-c", "advice.detachedHead=false", "checkout", "-q", "-f", "--detach", tag)
    git(a.src, "config", "user.name", "craft-web-overrides")
    git(a.src, "config", "user.email", "craft-web-overrides@users.noreply.github.com")

    applied, dropped = [], []
    for patch in patches:
        if "pr" in patch:
            pr = api(f"repos/{UPSTREAM}/{a.app}/pulls/{patch['pr']}")
            if pr.get("merged_at") and pr.get("merge_commit_sha") and is_ancestor(a.src, pr["merge_commit_sha"], tag):
                dropped.append({"patch": f"{UPSTREAM}/{a.app}#{patch['pr']}", "reason": f"merged, in {tag}"})
                continue
        label, commits = patch_commits(a.src, a.app, patch)
        # `git cherry` marks a commit whose change is already in the release with "-".
        present = {line[2:] for line in git(a.src, "cherry", "HEAD", commits[-1], f"{commits[0]}~1").stdout.splitlines() if line.startswith("- ")} if commits else set()
        todo = [c for c in commits if c not in present]
        if not todo:
            dropped.append({"patch": label, "reason": f"already in {tag}"})
            continue
        for c in todo:
            r = git(a.src, "cherry-pick", "-x", c, check=False)
            if r.returncode != 0:
                git(a.src, "cherry-pick", "--abort", check=False)
                sys.exit(f"{label}: commit {c[:10]} doesn't apply to {tag}; update the patch or drop it.\n{r.stdout}{r.stderr}")
        applied.append({"patch": label, "why": patch.get("why", ""), "commits": todo})

    libs = lib_uses(a.src, overrides.get("libs", {})) if applied else []
    version = tag.lstrip("v")
    inputs = [tag, [p["commits"] for p in applied]] + ([[u["commit"] for u in libs]] if libs else [])
    ident = hashlib.sha256(json.dumps(inputs).encode()).hexdigest()[:8]
    result = {"app": a.app, "tag": tag, "version": version, "id": ident, "applied": applied, "dropped": dropped, "libs": libs,
              "head": git(a.src, "rev-parse", "HEAD").stdout.strip(),
              "status": "patched" if applied else "upstream"}
    build = bool(applied) and (a.force or manifest.get(a.app, {}).get("id") != ident)
    result["build"] = build
    os.makedirs(a.out, exist_ok=True)
    json.dump(result, open(os.path.join(a.out, "result.json"), "w"), indent=2)
    print(json.dumps(result, indent=2), file=sys.stderr)
    print(f"build={'true' if build else 'false'}")
    print(f"version={version}")


if __name__ == "__main__":
    main()
