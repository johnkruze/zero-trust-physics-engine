#!/usr/bin/env python3
"""Merge build branches into an integration copy, crate by crate, and report.

    python3 kit/merge_branches.py <integration dir> <copy dir> [<copy dir> ...]
        [--crates genesis_kernel,genesis_solvers,...] [--dry-run] [--keep-conflicts]

Each <copy dir> is one build session's copy of the workspace (for example
genesis_workspace/.build_2026-10-14/wave1_josephson); in each crate there the
branch to merge is the one that copy has checked out (wave1-<name>, or
next-<name>). The integration dir is the copy the branches are merged into,
each crate a Git repository on its integration branch.

For every crate of every copy, in the order given:
  - nothing new: the copy's branch holds no commit the integration lacks;
  - merged: fetched and merged with a merge commit, no conflict;
  - CONFLICT: the files that conflict are listed, and the merge is undone
    (with --keep-conflicts it is left in place and the run stops, so the
    conflicts can be resolved in the integration copy by hand).

--dry-run merges nothing: each branch is tried against the integration's
current tip in memory (git merge-tree) and the report says what a merge
would do. No branch and no file of the integration copy is changed. A crate whose working tree
has changes not committed is skipped, never merged into.

Exit code 0 when every branch merged (or would) cleanly, 1 when any conflicts.
"""

import argparse
import os
import subprocess
import sys

CRATES = ["genesis_kernel", "genesis_solvers", "genesis_seal", "genesis_seal_parquet", "protocol_mcp_gateway"]


def git(repo, *args, check=True):
    done = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)
    if check and done.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} in {repo}: {done.stderr.strip() or done.stdout.strip()}")
    return done


def is_repo(path):
    return os.path.isdir(os.path.join(path, ".git"))


def merge_one(target, source, dry_run, keep):
    """One crate of one copy into the integration's same crate."""
    branch = git(source, "branch", "--show-current").stdout.strip()
    if not branch:
        return "skipped", f"{source} has no branch checked out", []
    if git(target, "status", "--porcelain", "--untracked-files=no").stdout.strip():
        return "skipped", "the integration crate has changes not committed", []
    # The copy is fetched by its absolute path: git reads a relative one
    # from the integration crate, where it names nothing.
    fetched = git(target, "fetch", "--no-tags", os.path.abspath(source), branch, check=False)
    if fetched.returncode != 0:
        return "skipped", f"could not fetch {branch}: {fetched.stderr.strip()}", []
    tip = git(target, "rev-parse", "FETCH_HEAD").stdout.strip()
    new = git(target, "rev-list", "--count", f"HEAD..{tip}").stdout.strip()
    if new == "0":
        return "nothing new", f"{branch} at {tip[:7]}", []
    if dry_run:
        tried = git(target, "merge-tree", "--write-tree", "--name-only", "--no-messages", "HEAD", tip, check=False)
        if tried.returncode == 0:
            return "would merge", f"{branch}, {new} commits", []
        lines = tried.stdout.strip().splitlines()
        return "CONFLICT", f"{branch}, {new} commits", lines[1:]
    message = f"Merge {branch} of {os.path.basename(os.path.dirname(os.path.abspath(source)))} into the integration."
    merged = git(target, "merge", "--no-ff", "--no-edit", "-m", message, tip, check=False)
    if merged.returncode == 0:
        return "merged", f"{branch}, {new} commits, now {git(target, 'rev-parse', '--short', 'HEAD').stdout.strip()}", []
    conflicted = git(target, "diff", "--name-only", "--diff-filter=U", check=False).stdout.split()
    if not keep:
        git(target, "merge", "--abort", check=False)
    return "CONFLICT", f"{branch}, {new} commits" + ("" if keep else ", merge undone"), conflicted


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("integration")
    parser.add_argument("copies", nargs="+")
    parser.add_argument("--crates", default=",".join(CRATES))
    parser.add_argument("--dry-run", action="store_true", help="merge nothing; say what a merge would do")
    parser.add_argument("--keep-conflicts", action="store_true", help="leave a conflicted merge in place and stop")
    args = parser.parse_args()
    crates = [c for c in args.crates.split(",") if c]
    rows, conflicts = [], 0
    for copy in args.copies:
        for crate in crates:
            source, target = os.path.join(copy, crate), os.path.join(args.integration, crate)
            if not (is_repo(source) and is_repo(target)):
                continue
            try:
                outcome, detail, files = merge_one(target, source, args.dry_run, args.keep_conflicts)
            except RuntimeError as error:
                outcome, detail, files = "skipped", str(error), []
            rows.append((os.path.basename(os.path.abspath(copy)), crate, outcome, detail, files))
            if outcome == "CONFLICT":
                conflicts += 1
                if args.keep_conflicts and not args.dry_run:
                    break
        if conflicts and args.keep_conflicts and not args.dry_run:
            break
    width = max([len(r[0]) for r in rows] + [4])
    for copy, crate, outcome, detail, files in rows:
        print(f"{copy:<{width}}  {crate:<22} {outcome:<12} {detail}")
        for name in files:
            print(f"{'':<{width}}  {'':<22} {'':<12}   conflicts: {name}")
    print(f"\n{len(rows)} crate branches: "
          + ", ".join(f"{sum(1 for r in rows if r[2] == o)} {o}" for o in ["merged", "would merge", "nothing new", "CONFLICT", "skipped"] if any(r[2] == o for r in rows)))
    return 1 if conflicts else 0


if __name__ == "__main__":
    sys.exit(main())
