#!/usr/bin/env python3
"""Checks out an app's newest storytold release and applies its patches from overrides.json.

    tools/prepare.py <app> --src <checkout dir> --out <result dir> [--manifest manifest.json] [--force]

Writes <result dir>/result.json and prints `build=true|false` for $GITHUB_OUTPUT:
- each patch is an upstream PR (`pr`), a branch (`branch`: "owner/repo:branch") or both; its commits are the
  non-merge commits between storytold's main and the patch head, cherry-picked in order;
- a patch whose PR merged before the release, or whose commits are all already in the release (same patch id),
  is dropped; when every patch is dropped the app needs no override (`status: upstream`);
- `id` hashes the release tag and the applied commits, so the same inputs never build twice: `build=false` when
  the manifest already has that id (unless --force).
A patch that doesn't apply fails the run; the mirror keeps serving the last good build.
Needs git, and GH_TOKEN for the GitHub API (only public data is read).
"""
import argparse, hashlib, json, os, subprocess, sys, urllib.request

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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("app")
    ap.add_argument("--src", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--manifest", default=os.path.join(ROOT, "manifest.json"))
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args()

    overrides = json.load(open(os.path.join(ROOT, "overrides.json")))
    patches = overrides[a.app]["patches"]
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

    version = tag.lstrip("v")
    ident = hashlib.sha256(json.dumps([tag, [p["commits"] for p in applied]]).encode()).hexdigest()[:8]
    result = {"app": a.app, "tag": tag, "version": version, "id": ident, "applied": applied, "dropped": dropped,
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
