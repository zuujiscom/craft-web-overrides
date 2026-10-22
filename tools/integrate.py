#!/usr/bin/env python3
"""Build a testing branch of a storytold app from the PRs that FredApps approved and that pass CI.

Usage:
    tools/integrate.py <app> --work <scratch dir> [--branch fredapps-approved-ci-green] [--report out.md] [--push]

The base is the mirror's `main` (zuujiscom/<app>), which fork sync keeps equal to storytold's newest green
main. Nothing here ever writes to `main`: the result is one branch, rebuilt from scratch on every run and
force-pushed only when --push is given.

A PR is merged only when
- it is open, not a draft, not by a bot, based on main;
- FredApps' latest review of it is an approval, given on the PR's current head commit (a later push drops it
  until it is reviewed again; a later request for changes drops it);
- CI on that head commit is green, CodeRabbit's check passed on it and CodeRabbit posted no actionable
  comments on it (findings on older commits don't count: the author has pushed since);
- it passes the skip list below, the paths and patterns where code runs at build time or in CI, or secrets and
  obfuscated payloads hide. A skipped PR is listed with its reason, never merged partly;
- it merges cleanly onto what is already in the branch (oldest approval first). Conflicts are listed, with files.

The merged commits are the PR heads FredApps approved, byte for byte. This script never builds or runs the code.
"""
import argparse, json, os, re, subprocess, sys, time

UPSTREAM = "storytold"
MIRROR = "zuujiscom"
REVIEWER = "FredApps"

# Paths where a change runs at build time, in CI or on a developer's machine, or changes what is fetched.
SKIP_PATHS = [
    r"^\.github/", r"(^|/)build\.rs$", r"^xtask/", r"^\.cargo/", r"^packaging/", r"^scripts?/", r"^tools/",
    r"(^|/)Cargo\.(toml|lock)$", r"(^|/)rust-toolchain(\.toml)?$", r"(^|/)deny\.toml$", r"(^|/)\.gitmodules$",
    r"(^|/)\.gitattributes$", r"\.(sh|bash|ps1|bat|cmd|py|rb|pl|js|mjs|cjs|ts)$", r"(^|/)Dockerfile", r"^(flake\.nix|flake\.lock)$",
    r"^nix/", r"(^|/)(package(-lock)?\.json|pnpm-lock\.yaml|yarn\.lock)$", r"(^|/)\.husky/", r"(^|/)Makefile$", r"(^|/)justfile$",
]
# Text in added lines that never belongs in a reviewed UI change.
SKIP_TEXT = [
    (r"(curl|wget)\s[^\n|]*\|\s*(ba|z)?sh", "pipes a download into a shell"),
    (r"base64\s+(-d|--decode)", "decodes base64 in a shell"),
    (r"\b(Invoke-Expression|iex)\b\s", "PowerShell Invoke-Expression"),
    (r"powershell(\.exe)?\s+.*-e(nc|ncodedcommand)", "PowerShell encoded command"),
    (r"/dev/tcp/", "bash network device"),
    (r"-----BEGIN [A-Z ]*PRIVATE KEY-----", "private key"),
    (r"\bAKIA[0-9A-Z]{16}\b", "AWS access key"),
    (r"\bgh[pousr]_[A-Za-z0-9]{30,}\b", "GitHub token"),
    (r"\bxox[abprs]-[A-Za-z0-9-]{10,}", "Slack token"),
    (r"[A-Za-z0-9+/=]{400,}", "a very long base64-like blob"),
]
# Worth a reviewer's eye but not a reason to skip: listed in the report.
FLAG_TEXT = [r"\bunsafe\b", r"Command::new", r"TcpStream", r"UdpSocket", r"reqwest|ureq|hyper::", r"include_bytes!", r"std::env::var", r"libloading|dlopen"]


def run(args, cwd=None, check=True, inp=None):
    r = subprocess.run(args, cwd=cwd, capture_output=True, text=True, input=inp)
    if check and r.returncode:
        sys.exit(f"{' '.join(args)}\n{r.stderr.strip()[:600]}")
    return r


def gh_graphql(query, **fields):
    args = ["gh", "api", "graphql", "-f", f"query={query}"]
    for k, v in fields.items():
        if v is not None:
            args += ["-f", f"{k}={v}"]
    for attempt in range(4):
        r = run(args, check=False)
        if r.returncode == 0:
            return json.loads(r.stdout)["data"]
        time.sleep(3 * (attempt + 1))
    sys.exit(f"graphql failed: {r.stderr.strip()[:400]}")


def candidates(app):
    """Open PRs whose latest FredApps review is an approval of the current head with green CI."""
    q = """query($after:String){repository(owner:"%s",name:"%s"){pullRequests(states:OPEN,first:30,after:$after,orderBy:{field:UPDATED_AT,direction:DESC}){
      pageInfo{hasNextPage endCursor}
      nodes{number title isDraft baseRefName headRefOid author{login __typename} reviews(last:20,author:"%s"){nodes{state submittedAt commit{oid}}}
      commits(last:1){nodes{commit{statusCheckRollup{state}}}}}}}}""" % (UPSTREAM, app, REVIEWER)
    out, why, after = [], {}, None

    def drop(reason):
        why[reason] = why.get(reason, 0) + 1

    while True:
        d = gh_graphql(q, after=after)["repository"]["pullRequests"]
        for p in d["nodes"]:
            if p["isDraft"]:
                drop("draft"); continue
            if p["baseRefName"] != "main":
                drop("not based on main"); continue
            if (p["author"] or {}).get("__typename") == "Bot":
                drop("bot author"); continue
            rv = p["reviews"]["nodes"]
            if not rv:
                drop("no review by " + REVIEWER); continue
            last = rv[-1]
            if last["state"] != "APPROVED":
                drop(f"latest review is {last['state'].lower()}"); continue
            if (last["commit"] or {}).get("oid") != p["headRefOid"]:
                drop("approval is not on the current head"); continue
            nodes = p["commits"]["nodes"]
            ci = ((nodes[0]["commit"]["statusCheckRollup"] or {}).get("state")) if nodes else None
            if ci != "SUCCESS":
                drop(f"CI is {str(ci).lower()}"); continue
            out.append({"number": p["number"], "title": p["title"], "head": p["headRefOid"], "author": (p["author"] or {}).get("login", "?"), "approved": last["submittedAt"]})
        if not d["pageInfo"]["hasNextPage"]:
            break
        after = d["pageInfo"]["endCursor"]
    kept = []
    for c, verdict in zip(out, coderabbit(app, [c["number"] for c in out])):
        if verdict:
            drop(verdict)
        else:
            kept.append(c)
    kept.sort(key=lambda c: (c["approved"], c["number"]))
    return kept, why


def coderabbit(app, numbers):
    """For each PR: None when CodeRabbit's check passed on the head and it posted no actionable comments
    on that head, else the reason. Findings on older commits don't count (the author pushed since)."""
    verdicts = []
    for i in range(0, len(numbers), 15):
        group = numbers[i:i + 15]
        parts = [f'p{n}:pullRequest(number:{n}){{headRefOid reviews(last:30){{nodes{{author{{login}} body commit{{oid}}}}}} '
                 f'commits(last:1){{nodes{{commit{{statusCheckRollup{{contexts(first:60){{nodes{{__typename ... on CheckRun{{name conclusion}} '
                 f'... on StatusContext{{context state}}}}}}}}}}}}}}}}' for n in group]
        repo = gh_graphql('query{repository(owner:"%s",name:"%s"){%s}}' % (UPSTREAM, app, " ".join(parts)))["repository"]
        for n in group:
            p = repo[f"p{n}"]
            ctx = [c for nd in p["commits"]["nodes"] for c in (((nd["commit"]["statusCheckRollup"] or {}).get("contexts") or {}).get("nodes") or [])]
            checks = [(c.get("conclusion") or c.get("state")) for c in ctx if "coderabbit" in (c.get("name") or c.get("context") or "").lower()]
            if "SUCCESS" not in checks:
                verdicts.append("no successful CodeRabbit check"); continue
            found = 0
            for r in p["reviews"]["nodes"]:
                if (r["author"] or {}).get("login") == "coderabbitai" and (r["commit"] or {}).get("oid") == p["headRefOid"]:
                    m = re.search(r"Actionable comments posted:\s*(\d+)", r["body"] or "")
                    found += int(m.group(1)) if m else 0
            verdicts.append(f"CodeRabbit has {found} actionable findings on the current head" if found else None)
    return verdicts


# Only the translation tables: both sides' rows are kept. Not TOML or generated docs: a union of those
# duplicates keys or leaves a generated file out of date (found by `cargo xtask scorecard --check`).
UNION_PATHS = ["crates/ui-egui/src/i18n/*.tsv"]


def duplicate_keys(work):
    """A key (first column) that appears twice in a translation catalog the last merge touched."""
    names = run(["git", "diff", "--name-only", "HEAD~1", "HEAD", "--", "*.tsv"], cwd=work, check=False).stdout.split()
    for name in names:
        seen = set()
        for line in run(["git", "show", f"HEAD:{name}"], cwd=work, check=False).stdout.splitlines():
            if not line or line.startswith("#"):
                continue
            key = line.split("\t", 1)[0]
            if not key.strip() or key.startswith("@"):
                continue
            if key in seen:
                return f"{key!r} in {name}"
            seen.add(key)
    return None


def proc_macro_dirs(work):
    """Directories of crates that are procedural macros at the base (they run inside the compiler)."""
    r = run(["git", "grep", "-l", "-E", r"proc-macro\s*=\s*true", "HEAD", "--", "*Cargo.toml"], cwd=work, check=False)
    dirs = []
    for line in r.stdout.splitlines():
        path = line.split(":", 1)[-1]
        dirs.append(os.path.dirname(path))
    return dirs


def inspect(work, base, c, macro_dirs):
    """Skip reasons and flags for one fetched PR (`refs/pr/<n>`)."""
    ref = f"refs/pr/{c['number']}"
    mb = run(["git", "merge-base", base, ref], cwd=work, check=False).stdout.strip()
    if not mb:
        return ["no common history with main"], []
    reasons, flags = [], []
    raw = run(["git", "diff", "--raw", "--no-renames", mb, ref], cwd=work).stdout.splitlines()
    paths = []
    for line in raw:
        meta, _, path = line.partition("\t")
        paths.append(path)
        modes = meta.split()
        if "120000" in modes[:2]:
            reasons.append(f"adds or changes a symlink: {path}")
        if "160000" in modes[:2]:
            reasons.append(f"changes a submodule: {path}")
    for path in paths:
        for pat in SKIP_PATHS:
            if re.search(pat, path):
                reasons.append(f"touches {path}"); break
        for d in macro_dirs:
            if d and path.startswith(d + "/"):
                reasons.append(f"touches procedural-macro crate {d}"); break
    for line in run(["git", "diff", "--numstat", "--no-renames", mb, ref], cwd=work).stdout.splitlines():
        a, b, path = line.split("\t", 2)
        if a == "-" and b == "-":
            reasons.append(f"binary file: {path}")
    diff = run(["git", "diff", "-U0", "--no-renames", mb, ref], cwd=work).stdout
    added = "\n".join(l[1:] for l in diff.splitlines() if l.startswith("+") and not l.startswith("+++"))
    for pat, what in SKIP_TEXT:
        m = re.search(pat, added)
        if m:
            reasons.append(f"added text {what}")
    for pat in FLAG_TEXT:
        if re.search(pat, added):
            flags.append(pat.replace("\\b", ""))
    return sorted(set(reasons)), flags


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("app")
    ap.add_argument("--work", required=True)
    ap.add_argument("--branch", default="fredapps-approved-ci-green")
    ap.add_argument("--report")
    ap.add_argument("--push", action="store_true")
    ap.add_argument("--deny", help="JSON {pr number: reason} of PRs to leave out (default testing/<app>-deny.json)")
    a = ap.parse_args()
    deny_path = a.deny or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "testing", f"{a.app}-deny.json")
    deny = json.load(open(deny_path)) if os.path.exists(deny_path) else {}
    if a.branch in ("main", "master", "release"):
        sys.exit("refusing to write to " + a.branch)

    cands, dropped = candidates(a.app)
    print(f"{len(cands)} PRs approved on their head with green CI; not eligible: {dropped}", file=sys.stderr)

    work = a.work
    if not os.path.isdir(os.path.join(work, ".git")):
        run(["git", "clone", "-q", "--filter=blob:none", "--no-checkout", f"https://github.com/{MIRROR}/{a.app}.git", work])
    run(["git", "remote", "remove", "upstream"], cwd=work, check=False)
    run(["git", "remote", "add", "upstream", f"https://github.com/{UPSTREAM}/{a.app}.git"], cwd=work)
    # PR heads can need objects only storytold has: let the partial clone fetch missing blobs from it too.
    run(["git", "config", "remote.upstream.promisor", "true"], cwd=work)
    run(["git", "config", "remote.upstream.partialclonefilter", "blob:none"], cwd=work)
    run(["git", "config", "user.name", "zuujiscom"], cwd=work)
    run(["git", "config", "user.email", "zuujis@gmail.com"], cwd=work)
    run(["git", "config", "commit.gpgsign", "false"], cwd=work)
    run(["git", "fetch", "-q", "origin", "main"], cwd=work)
    run(["git", "fetch", "-q", "upstream", "main:refs/upstream-main"], cwd=work)
    base = run(["git", "rev-parse", "origin/main"], cwd=work).stdout.strip()
    behind = run(["git", "rev-list", "--count", f"{base}..refs/upstream-main"], cwd=work).stdout.strip()
    print(f"base = mirror main {base[:8]} ({behind} commits behind storytold main)", file=sys.stderr)
    run(["git", "checkout", "-q", "-B", "work", base], cwd=work)

    for i in range(0, len(cands), 80):
        specs = [f"+refs/pull/{c['number']}/head:refs/pr/{c['number']}" for c in cands[i:i + 80]]
        run(["git", "fetch", "-q", "upstream", *specs], cwd=work)
    # Never merge a head other than the one that was approved.
    for c in cands:
        got = run(["git", "rev-parse", f"refs/pr/{c['number']}"], cwd=work).stdout.strip()
        if got != c["head"]:
            c["skip"] = [f"head moved since the review ({got[:8]} vs {c['head'][:8]})"]
        if str(c["number"]) in deny:
            c["skip"] = c.get("skip", []) + ["deny list: " + deny[str(c["number"])]]

    # Files every PR adds its own rows to (translation catalogs): keep both sides' lines.
    # Translation catalogs are checked for a key added twice after each merge.
    with open(os.path.join(work, ".git", "info", "attributes"), "w") as f:
        f.write("\n".join(f"{p} merge=union" for p in UNION_PATHS) + "\n")

    macro = proc_macro_dirs(work)
    merged, skipped, conflicts = [], [], []
    for c in cands:
        reasons, flags = ([], []) if "skip" in c else inspect(work, base, c, macro)
        reasons = c.get("skip", []) + reasons
        if reasons:
            skipped.append({**c, "reasons": reasons}); continue
        r = run(["git", "merge", "-q", "--no-ff", "--no-edit", "-m",
                 f"Merge {UPSTREAM}/{a.app}#{c['number']} at {c['head'][:10]}: {c['title']}\n\nApproved by {REVIEWER} on this commit, CI green.", f"refs/pr/{c['number']}"], cwd=work, check=False)
        if r.returncode:
            files = run(["git", "diff", "--name-only", "--diff-filter=U"], cwd=work, check=False).stdout.split()
            run(["git", "merge", "--abort"], cwd=work, check=False)
            # Does it conflict with main alone, or only with PRs merged before it?
            alone = run(["git", "merge-tree", "--write-tree", "--no-messages", base, f"refs/pr/{c['number']}"], cwd=work, check=False).returncode
            conflicts.append({**c, "files": files[:8], "vs_main": alone != 0}); continue
        dup = duplicate_keys(work)
        if dup:
            run(["git", "reset", "-q", "--hard", "HEAD~1"], cwd=work)
            conflicts.append({**c, "files": [f"duplicate key {dup}"]}); continue
        merged.append({**c, "flags": flags})
        if len(merged) % 50 == 0:
            print(f"  merged {len(merged)}", file=sys.stderr)

    # Workflows always come from main, whatever a merged PR did (none should touch them).
    if run(["git", "diff", "--quiet", base, "--", ".github"], cwd=work, check=False).returncode:
        run(["git", "checkout", base, "--", ".github"], cwd=work)
        run(["git", "commit", "-q", "-m", "Restore .github from main"], cwd=work)
    head = run(["git", "rev-parse", "HEAD"], cwd=work).stdout.strip()

    lines = [f"# {a.app}: {a.branch}", "",
             f"Built {time.strftime('%Y-%m-%d %H:%M UTC', time.gmtime())} on mirror main `{base[:10]}` ({behind} behind storytold main). Result `{head[:10]}`.", "",
             f"- merged: **{len(merged)}**",
             f"- skipped by the skip list: **{len(skipped)}**",
             f"- conflicts: **{len(conflicts)}** ({sum(1 for s in conflicts if s.get('vs_main'))} with main itself, {sum(1 for s in conflicts if not s.get('vs_main'))} only with PRs merged before them)",
             f"- not eligible (review, CI, draft...): {json.dumps(dropped)}", ""]
    lines += ["## Skipped", ""] + [f"- #{s['number']} {s['title'][:70]} ({s['author']}): {'; '.join(s['reasons'])[:200]}" for s in skipped]
    lines += ["", "## Conflicts", ""] + [f"- #{s['number']} {s['title'][:70]}: {', '.join(s['files'])}" for s in conflicts]
    lines += ["", "## Merged (oldest approval first; flags are for a closer look)", ""] + \
             [f"- #{m['number']} {m['title'][:70]} ({m['author']})" + (f" [flags: {', '.join(m['flags'])}]" if m["flags"] else "") for m in merged]
    report = "\n".join(lines) + "\n"
    if a.report:
        open(a.report, "w").write(report)
    print("\n".join(lines[:9]))

    if a.push:
        # The push lazily fetches blobs it lacks from origin, which never has blobs that only storytold's PR heads carry.
        missing = [l[1:] for l in run(["git", "rev-list", "--objects", "--missing=print", f"{base}..HEAD"], cwd=work).stdout.split("\n") if l.startswith("?")]
        for i in range(0, len(missing), 200):
            run(["git", "fetch", "-q", "--no-write-fetch-head", "upstream", *missing[i:i + 200]], cwd=work)
        run(["git", "push", "-q", "--force", "origin", f"HEAD:refs/heads/{a.branch}"], cwd=work)
        print(f"pushed {MIRROR}/{a.app}:{a.branch} at {head[:10]}")
    json.dump({"base": base, "head": head, "merged": [m["number"] for m in merged]}, open(os.path.join(work, ".git", "integrate.json"), "w"))


if __name__ == "__main__":
    main()
