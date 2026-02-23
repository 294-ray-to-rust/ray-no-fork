#!/usr/bin/env python3
"""
ray-agents orchestrator

Runs a loop: Manager creates issues -> SWE implements them -> repeat.
Each SWE run works in a git worktree on a fresh branch so changes are isolated.

Usage:
    ./run.py                                        # Defaults: 5 cycles, 3 SWE runs each
    ./run.py --max-cycles 10                        # 10 manager cycles
    ./run.py --swe-runs-per-cycle 5                 # 5 SWE runs between manager runs
    ./run.py --base-branch main                     # Branch to fork from (default: main)
    ./run.py --worktree-dir /tmp/ray-agents-work    # Where to put worktrees
    ./run.py --resume                               # Resume from last saved state
"""

import argparse
import json
import logging
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)-5s %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Preflight checks
# ---------------------------------------------------------------------------

def preflight(project_dir: Path) -> None:
    """Verify everything is in place before starting."""
    errors = []

    if not shutil.which("opencode"):
        errors.append("opencode is not installed. Install from https://opencode.ai")

    if not shutil.which("gh"):
        errors.append("gh CLI is not installed. Install from https://cli.github.com")

    if not shutil.which("git"):
        errors.append("git is not installed.")

    goal = project_dir / "goal.md"
    if not goal.exists() or goal.stat().st_size == 0:
        errors.append("goal.md is missing or empty. Write your project goal first.")

    if subprocess.run(["gh", "auth", "status"], cwd=project_dir, capture_output=True, text=True).returncode != 0:
        errors.append("gh CLI is not authenticated. Run: gh auth login")

    if subprocess.run(["git", "remote", "get-url", "origin"], cwd=project_dir, capture_output=True, text=True).returncode != 0:
        errors.append(
            "No git remote 'origin' configured.\n"
            "Create a GitHub repo and run: git remote add origin <url>"
        )

    if errors:
        for e in errors:
            log.error(e)
        sys.exit(1)


# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

MEMORY_SEED = """\
# Project Memory

## Status
- **Project status**: not_started
- **Last run**: never
- **Run count**: 0

## Summary
No work has been done yet.

## Completed Issues
None yet.

## Active Issues
None yet.

## Decisions
None yet.

## Blockers
None yet.

## Next Priorities
None yet.
"""


def ensure_memory(project_dir: Path) -> None:
    memory = project_dir / "memory.md"
    if not memory.exists():
        memory.write_text(MEMORY_SEED)
        log.info("Created initial memory.md")


def ensure_labels(project_dir: Path) -> None:
    log.info("Ensuring GitHub labels exist...")
    for label in ("ready", "in-progress", "blocked", "completed", "needs-review"):
        subprocess.run(["gh", "label", "create", label, "--force"], cwd=project_dir, capture_output=True, text=True)


# ---------------------------------------------------------------------------
# Git worktree management
# ---------------------------------------------------------------------------

def create_worktree(
    project_dir: Path,
    worktree_base: Path,
    branch_name: str,
    base_branch: str,
) -> Path:
    """Create a git worktree on a new branch forked from base_branch.

    Returns the path to the worktree directory.
    """
    worktree_path = worktree_base / branch_name

    # Clean up if a stale worktree exists at this path
    if worktree_path.exists():
        log.info("Cleaning up stale worktree at %s", worktree_path)
        subprocess.run(["git", "worktree", "remove", "--force", str(worktree_path)], cwd=project_dir, capture_output=True, text=True)
        if worktree_path.exists():
            shutil.rmtree(worktree_path)

    # Delete the branch if it already exists (leftover from previous run)
    if subprocess.run(["git", "rev-parse", "--verify", branch_name], cwd=project_dir, capture_output=True, text=True).returncode == 0:
        subprocess.run(["git", "branch", "-D", branch_name], cwd=project_dir, capture_output=True, text=True)

    # Create the worktree with a new branch
    worktree_base.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "worktree", "add", "-b", branch_name, str(worktree_path), base_branch],
        cwd=project_dir, capture_output=True, text=True, check=True)

    # Copy current config files into the worktree so it gets the latest
    # settings even if they haven't been committed to the base branch yet.
    for config in ("opencode.json",):
        src = project_dir / config
        if src.exists():
            shutil.copy2(src, worktree_path / config)
    opencode_dir = project_dir / ".opencode"
    if opencode_dir.is_dir():
        dst = worktree_path / ".opencode"
        if dst.exists():
            shutil.rmtree(dst)
        shutil.copytree(opencode_dir, dst, dirs_exist_ok=True)

    log.info("Created worktree: %s (branch: %s)", worktree_path, branch_name)
    return worktree_path


def remove_worktree(project_dir: Path, worktree_path: Path) -> None:
    """Remove a git worktree."""
    subprocess.run(["git", "worktree", "remove", "--force", str(worktree_path)], cwd=project_dir, capture_output=True, text=True)
    if worktree_path.exists():
        shutil.rmtree(worktree_path, ignore_errors=True)
    subprocess.run(["git", "worktree", "prune"], cwd=project_dir, capture_output=True, text=True)


def has_commits_ahead(worktree_path: Path, base_branch: str) -> bool:
    """Check if the worktree branch has commits ahead of base_branch."""
    result = subprocess.run(["git", "rev-list", "--count", f"{base_branch}..HEAD"], cwd=worktree_path, capture_output=True, text=True)
    return result.returncode == 0 and int(result.stdout.strip() or "0") > 0


# ---------------------------------------------------------------------------
# Push & PR helpers
# ---------------------------------------------------------------------------

def push_branch(project_dir: Path, branch_name: str) -> bool:
    """Push a local branch to origin. Returns True on success."""
    result = subprocess.run(
        ["git", "push", "--set-upstream", "origin", branch_name],
        cwd=project_dir, capture_output=True, text=True,
    )
    if result.returncode != 0:
        log.error("Failed to push branch '%s': %s", branch_name, result.stderr.strip())
        return False
    log.info("Pushed branch '%s' to origin.", branch_name)
    return True


def extract_issue_number(project_dir: Path, branch_name: str, base_branch: str) -> int | None:
    """Extract the issue number from commit messages on a branch.

    Looks for patterns like '#42' in commit subjects between base_branch and branch tip.
    Returns the first issue number found, or None.
    """
    result = subprocess.run(
        ["git", "log", "--format=%s", f"{base_branch}..{branch_name}"],
        cwd=project_dir, capture_output=True, text=True,
    )
    if result.returncode != 0:
        return None
    for line in result.stdout.strip().splitlines():
        match = re.search(r"#(\d+)", line)
        if match:
            return int(match.group(1))
    return None


def create_pull_request(
    project_dir: Path,
    branch_name: str,
    base_branch: str,
    issue_number: int | None,
    draft: bool = False,
) -> str | None:
    """Create a GitHub PR for the branch. Returns the PR URL or None on failure."""
    if issue_number:
        title = f"Implement #{issue_number}"
        body = (
            f"Closes #{issue_number}\n\n"
            f"## Branch\n`{branch_name}`\n\n"
            f"## Summary\n"
            f"Automated PR created by the orchestrator for work on issue #{issue_number}.\n"
            f"See the linked issue for acceptance criteria and context.\n"
        )
    else:
        title = f"SWE work: {branch_name}"
        body = (
            f"## Branch\n`{branch_name}`\n\n"
            f"## Summary\n"
            f"Automated PR created by the orchestrator. "
            f"No linked issue was found in commit messages.\n"
        )

    cmd = [
        "gh", "pr", "create",
        "--base", base_branch,
        "--head", branch_name,
        "--title", title,
        "--body", body,
    ]
    if draft:
        cmd.append("--draft")
    else:
        cmd.extend(["--label", "needs-review"])

    result = subprocess.run(cmd, cwd=project_dir, capture_output=True, text=True)
    if result.returncode != 0:
        log.error("Failed to create PR for '%s': %s", branch_name, result.stderr.strip())
        return None

    pr_url = result.stdout.strip()
    log.info("Created PR: %s", pr_url)

    # Explicitly link the PR to the issue in GitHub's sidebar
    if issue_number:
        subprocess.run(
            ["gh", "issue", "develop", "--issue", str(issue_number),
             "--base", base_branch, "--name", branch_name],
            cwd=project_dir, capture_output=True, text=True,
        )

    return pr_url


def is_issue_closed(project_dir: Path, issue_number: int) -> bool:
    """Check if a GitHub issue is closed."""
    result = subprocess.run(
        ["gh", "issue", "view", str(issue_number), "--json", "state", "--jq", ".state"],
        cwd=project_dir, capture_output=True, text=True,
    )
    return result.returncode == 0 and result.stdout.strip() == "CLOSED"


def reset_issue_to_ready(project_dir: Path, issue_number: int) -> None:
    """Reset an in-progress issue back to ready (SWE didn't finish)."""
    subprocess.run(
        ["gh", "issue", "edit", str(issue_number),
         "--remove-label", "in-progress", "--add-label", "ready"],
        cwd=project_dir, capture_output=True, text=True,
    )
    subprocess.run(
        ["gh", "issue", "comment", str(issue_number),
         "--body", "SWE agent did not complete this issue. Resetting to ready. A draft PR with partial progress has been created."],
        cwd=project_dir, capture_output=True, text=True,
    )
    log.info("Reset issue #%d to ready.", issue_number)


def pull_base_branch(project_dir: Path, base_branch: str) -> None:
    """Pull the latest base branch so new worktrees include merged PRs."""
    result = subprocess.run(
        ["git", "pull", "origin", base_branch],
        cwd=project_dir, capture_output=True, text=True,
    )
    if result.returncode != 0:
        log.warning("Failed to pull %s: %s", base_branch, result.stderr.strip())
    else:
        log.info("Pulled latest '%s'.", base_branch)


# ---------------------------------------------------------------------------
# Agent runners
# ---------------------------------------------------------------------------

MANAGER_PROMPT = """\
You are the Manager Agent. Execute your full protocol now:
1. Read goal.md and memory.md
2. Query all open GitHub issues with: gh issue list --state open --json number,title,labels,body,comments --limit 50
3. Query recently closed issues with: gh issue list --state closed --json number,title,labels --limit 20 --search 'sort:updated-desc'
4. Analyze the state: count ready/in-progress/blocked issues, identify gaps vs the goal
5. Unblock any blocked issues if possible (highest priority)
6. Create new ready issues for work that has no issue yet (target: 2-3 ready issues)
7. Close any stale or irrelevant issues
8. Update memory.md with your decisions and the current timestamp
9. Print a summary of actions taken"""

SWE_PROMPT = """\
You are the SWE Agent. Execute your full protocol now:
1. Check for draft PRs first: gh pr list --state open --draft --json number,title,body,headRefName,url --limit 10
2. Query for ready issues: gh issue list --label ready --state open --json number,title,body --jq 'sort_by(.number)' --limit 10
3. If no ready issues, print 'No ready issues available. Exiting.' and stop
4. Prioritize issues with an existing draft PR (continue previous work), then lowest-numbered
5. If continuing a draft PR, read its diff to understand what was already done
6. Claim it (change label to in-progress, add a comment)
7. Read goal.md to see overarching goal
8. Understand the task from the issue body and codebase exploration
9. Implement the solution
10. Validate (run tests if they exist)
11. Report results: commit and close if done, or mark blocked with explanation"""


def run_agent(
    agent: str,
    title: str,
    prompt: str,
    cwd: Path,
    log_file: Path,
) -> int:
    """Run an opencode agent, tee-ing output to a log file. Returns exit code."""
    cmd = [
        "opencode", "run",
        "--agent", agent,
        "--title", title,
        prompt,
    ]

    log_file.parent.mkdir(parents=True, exist_ok=True)

    with open(log_file, "w") as lf:
        proc = subprocess.Popen(
            cmd,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        for line in proc.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            lf.write(line)
        proc.wait()

    return proc.returncode


def run_manager(cycle: int, timestamp: str, project_dir: Path, log_dir: Path) -> int:
    """Run the manager agent in the main project directory."""
    log_file = log_dir / f"{timestamp}_cycle{cycle}_manager.log"

    print()
    print(f"--- MANAGER (cycle {cycle}) ---")
    log.info("Starting manager agent...")

    title = f"manager-cycle-{cycle}-{timestamp}"
    exit_code = run_agent("manager", title, MANAGER_PROMPT, project_dir, log_file)

    log.info("Manager finished (exit code: %d)", exit_code)
    return exit_code


def run_swe(
    cycle: int,
    run_num: int,
    timestamp: str,
    worktree_path: Path,
    log_dir: Path,
) -> int:
    """Run the SWE agent inside a git worktree."""
    log_file = log_dir / f"{timestamp}_cycle{cycle}_swe{run_num}.log"

    print()
    print(f"--- SWE (cycle {cycle}, run {run_num}) ---")
    log.info("Starting SWE agent in worktree: %s", worktree_path)

    title = f"swe-cycle-{cycle}-run-{run_num}-{timestamp}"
    exit_code = run_agent("swe", title, SWE_PROMPT, worktree_path, log_file)

    log.info("SWE finished (exit code: %d)", exit_code)
    return exit_code


REVIEWER_PROMPT = """\
You are the Reviewer Agent. Execute your full protocol now:
1. List open PRs labeled 'needs-review': gh pr list --label needs-review --state open --json number,title,body,url --limit 20
2. If no PRs need review, print 'No PRs to review. Exiting.' and stop
3. For each PR:
   a. Read the diff: gh pr diff <NUMBER>
   b. Read the linked issue (from PR body) for acceptance criteria
   c. Check code quality, correctness, and whether acceptance criteria are met
   d. Either approve+merge or request changes with specific feedback
4. Print a summary of all review actions taken"""


def run_reviewer(
    cycle: int,
    timestamp: str,
    project_dir: Path,
    log_dir: Path,
) -> int:
    """Run the reviewer agent in the main project directory."""
    log_file = log_dir / f"{timestamp}_cycle{cycle}_reviewer.log"

    print()
    print(f"--- REVIEWER (cycle {cycle}) ---")
    log.info("Starting reviewer agent...")

    title = f"reviewer-cycle-{cycle}-{timestamp}"
    exit_code = run_agent("reviewer", title, REVIEWER_PROMPT, project_dir, log_file)

    log.info("Reviewer finished (exit code: %d)", exit_code)
    return exit_code


# ---------------------------------------------------------------------------
# State checks
# ---------------------------------------------------------------------------

def issue_count(project_dir: Path, label: str) -> int:
    """Count open issues with a given label."""
    result = subprocess.run(
        ["gh", "issue", "list", "--label", label, "--state", "open",
         "--json", "number", "--jq", "length"],
        cwd=project_dir, capture_output=True, text=True,
    )
    return int(result.stdout.strip() or "0") if result.returncode == 0 else 0


def check_all_blocked(project_dir: Path) -> bool:
    """Return True if there are no actionable issues (no ready, no in-progress)."""
    return issue_count(project_dir, "ready") == 0 and issue_count(project_dir, "in-progress") == 0


def check_project_completed(project_dir: Path) -> bool:
    """Return True if memory.md says the project is completed."""
    memory = project_dir / "memory.md"
    if not memory.exists():
        return False
    content = memory.read_text()
    return "**Project status**: completed" in content


# ---------------------------------------------------------------------------
# State persistence (for --resume)
# ---------------------------------------------------------------------------

STATE_FILE = ".opencode/state.json"


def load_state(project_dir: Path) -> dict | None:
    path = project_dir / STATE_FILE
    if not path.exists():
        return None
    with open(path) as f:
        return json.load(f)


def save_state(project_dir: Path, state: dict) -> None:
    path = project_dir / STATE_FILE
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(state, f, indent=2)


def clear_state(project_dir: Path) -> None:
    path = project_dir / STATE_FILE
    if path.exists():
        path.unlink()


# ---------------------------------------------------------------------------
# Main orchestration loop
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="ray-agents orchestrator: Manager + SWE agent loop with git worktrees"
    )
    parser.add_argument(
        "--max-cycles", type=int, default=5,
        help="Number of manager/SWE cycles (default: 5)",
    )
    parser.add_argument(
        "--swe-runs-per-cycle", type=int, default=3,
        help="SWE agent runs per cycle (default: 3)",
    )
    parser.add_argument(
        "--base-branch", type=str, default="main",
        help="Branch to fork SWE worktrees from (default: main)",
    )
    parser.add_argument(
        "--worktree-dir", type=str, default=None,
        help="Directory for git worktrees (default: .worktrees/ next to project)",
    )
    parser.add_argument(
        "--resume", action="store_true",
        help="Resume from last saved state instead of starting fresh",
    )
    args = parser.parse_args()

    project_dir = Path(__file__).resolve().parent
    os.chdir(project_dir)

    log_dir = project_dir / ".opencode" / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)

    worktree_base = (
        Path(args.worktree_dir) if args.worktree_dir
        else project_dir / ".worktrees"
    )

    # -----------------------------------------------------------------------
    # Resume or fresh start
    # -----------------------------------------------------------------------
    start_cycle = 1
    start_swe_run = 1
    skip_manager = False
    created_branches: list[str] = []

    if args.resume:
        state = load_state(project_dir)
        if state:
            start_cycle = state.get("cycle", 1)
            start_swe_run = state.get("swe_run", 1)
            skip_manager = state.get("manager_done", False)
            created_branches = state.get("created_branches", [])
            timestamp = state.get("timestamp", datetime.now().strftime("%Y%m%d_%H%M%S"))
            log.info("Resuming from cycle %d, swe_run %d (manager_done=%s)",
                     start_cycle, start_swe_run, skip_manager)
        else:
            log.info("No saved state found. Starting fresh.")
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    else:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    # -----------------------------------------------------------------------
    # Preflight
    # -----------------------------------------------------------------------
    preflight(project_dir)
    ensure_memory(project_dir)
    ensure_labels(project_dir)

    print("=" * 60)
    print("  ray-agents orchestrator")
    print(f"  Project:       {project_dir}")
    print(f"  Base branch:   {args.base_branch}")
    print(f"  Worktrees:     {worktree_base}")
    print(f"  Cycles:        {args.max_cycles}")
    print(f"  SWE runs:      {args.swe_runs_per_cycle} per cycle")
    print(f"  Logs:          {log_dir}")
    if args.resume and start_cycle > 1 or start_swe_run > 1:
        print(f"  Resuming from: cycle {start_cycle}, swe_run {start_swe_run}")
    print(f"  Started:       {datetime.now()}")
    print("=" * 60)

    for cycle in range(start_cycle, args.max_cycles + 1):
        print()
        print(f"============ CYCLE {cycle} / {args.max_cycles} ============")

        # --- Phase 1: Manager (runs in main project dir) ---
        if skip_manager and cycle == start_cycle:
            log.info("Skipping manager (already completed in previous run).")
        else:
            save_state(project_dir, {
                "cycle": cycle, "swe_run": 1, "manager_done": False,
                "timestamp": timestamp, "created_branches": created_branches,
            })

            exit_code = run_manager(cycle, timestamp, project_dir, log_dir)
            if exit_code != 0:
                log.warning("Manager exited with error. Continuing to SWE phase.")

        if check_project_completed(project_dir):
            print()
            log.info("Manager marked project as completed. Stopping.")
            clear_state(project_dir)
            break

        # --- Phase 2: SWE runs, each followed by push + PR + review ---
        first_swe = start_swe_run if cycle == start_cycle else 1

        for swe_run in range(first_swe, args.swe_runs_per_cycle + 1):

            save_state(project_dir, {
                "cycle": cycle, "swe_run": swe_run, "manager_done": True,
                "timestamp": timestamp, "created_branches": created_branches,
            })

            # Check for actionable work
            if check_all_blocked(project_dir):
                print()
                log.info("No ready or in-progress issues. All blocked or done.")
                log.info("Returning to manager early.")
                break

            # Create a branch and worktree for this SWE run
            branch_name = f"swe/cycle-{cycle}-run-{swe_run}-{timestamp}"
            worktree_path = create_worktree(
                project_dir, worktree_base, branch_name, args.base_branch
            )

            try:
                exit_code = run_swe(
                    cycle, swe_run, timestamp, worktree_path, log_dir
                )

                if exit_code != 0:
                    log.warning("SWE exited with error on run %d. Continuing.", swe_run)

                # Commit any uncommitted changes the SWE agent left behind
                status = subprocess.run(
                    ["git", "status", "--porcelain"],
                    cwd=worktree_path, capture_output=True, text=True,
                )
                if status.returncode == 0 and status.stdout.strip():
                    log.info("SWE left uncommitted changes. Auto-committing.")
                    subprocess.run(["git", "add", "-A"], cwd=worktree_path, capture_output=True, text=True)
                    subprocess.run(
                        ["git", "commit", "-m", "WIP: uncommitted changes from SWE agent"],
                        cwd=worktree_path, capture_output=True, text=True,
                    )

                # Check if the branch has any commits
                has_work = has_commits_ahead(worktree_path, args.base_branch)

            finally:
                # Always clean up the worktree (branch stays if it has commits)
                remove_worktree(project_dir, worktree_path)

            # Push, create PR, and run reviewer immediately after each SWE run
            if has_work:
                log.info("Branch '%s' has new commits.", branch_name)
                created_branches.append(branch_name)

                if push_branch(project_dir, branch_name):
                    issue_num = extract_issue_number(project_dir, branch_name, args.base_branch)
                    issue_complete = issue_num is not None and is_issue_closed(project_dir, issue_num)

                    if issue_complete:
                        # SWE finished the work — full PR + review
                        pr_url = create_pull_request(project_dir, branch_name, args.base_branch, issue_num)
                        if pr_url:
                            exit_code = run_reviewer(cycle, timestamp, project_dir, log_dir)
                            if exit_code != 0:
                                log.warning("Reviewer exited with error. PR remains open.")
                            pull_base_branch(project_dir, args.base_branch)
                    else:
                        # SWE didn't finish — draft PR + reset issue to ready
                        log.info("Issue not closed. Creating draft PR for partial work.")
                        create_pull_request(project_dir, branch_name, args.base_branch, issue_num, draft=True)
                        if issue_num is not None:
                            reset_issue_to_ready(project_dir, issue_num)
            else:
                log.info("No commits on '%s'. Cleaning up.", branch_name)

            time.sleep(3)

        # Save state pointing to next cycle
        save_state(project_dir, {
            "cycle": cycle + 1, "swe_run": 1, "manager_done": False,
            "timestamp": timestamp, "created_branches": created_branches,
        })

        print()
        log.info("Cycle %d complete.", cycle)
        time.sleep(2)
    else:
        # Loop completed without break — clear state
        clear_state(project_dir)

    # -----------------------------------------------------------------------
    # Final summary
    # -----------------------------------------------------------------------
    print()
    print("=" * 60)
    print("  Orchestration finished.")
    print(f"  Total cycles:  {args.max_cycles}")
    print(f"  Logs:          {log_dir}")
    print(f"  Finished:      {datetime.now()}")
    print("=" * 60)

    if check_project_completed(project_dir):
        print("  Status: PROJECT COMPLETED")
        clear_state(project_dir)
    else:
        print("  Status: Stopped (cycle limit reached or manual interruption)")
        print(f"  To resume: ./run.py --resume")

    if created_branches:
        print()
        print("  Branches with work (PRs created automatically):")
        for b in created_branches:
            print(f"    - {b}")
        print()
        print("  Check GitHub for merged PRs and any with requested changes.")
    else:
        print()
        print("  No branches with commits were created.")


if __name__ == "__main__":
    main()
