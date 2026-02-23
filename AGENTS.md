# Project: ray-agents

## Multi-Agent System

This project is managed by three AI agents coordinated through GitHub issues and pull requests:
- **Manager Agent**: Creates and manages issues based on the project goal. Never writes code.
- **SWE Agent**: Picks up issues and implements them. Never manages issues beyond commenting.
- **Reviewer Agent**: Reviews pull requests, approves and merges good work, or requests changes. Never writes code.

## GitHub Issue Protocol

### Labels
| Label | Meaning |
|-------|---------|
| `ready` | Issue is available for the SWE agent to pick up |
| `in-progress` | SWE agent is actively working on this issue |
| `blocked` | SWE agent could not complete; needs manager attention |
| `completed` | Work is finished; issue will be closed |
| `needs-review` | PR is waiting for the Reviewer agent to review |

### Issue Body Format
Every issue must contain:
1. **Objective**: What needs to be done and why
2. **Acceptance Criteria**: Checkboxes defining "done"
3. **Context**: Relevant file paths, related issues, technical details
4. **Dependencies**: Other issues that must be completed first, or "None"

### Comment Conventions
- SWE starting work: "Claimed. Starting implementation."
- SWE blocked: "BLOCKED: <reason>"
- SWE done: "Implementation complete. ... Closing."
- Manager unblocking: "Unblocked: <explanation>"

### Pull Request Protocol
- The orchestrator pushes branches and creates PRs after SWE agent commits
- PRs are labeled `needs-review` when created
- The Reviewer agent processes all `needs-review` PRs each cycle
- Approved PRs are squash-merged and their branches deleted
- PRs needing changes get a review comment and a new `ready` issue is created
- PRs reference their source issue with "Closes #N" in the body

## Code Conventions
- Commit messages reference issue numbers: "Implement #42: Add user auth"
- No force pushes
- No direct pushes from agents (the orchestrator pushes branches and creates PRs)
- Follow existing code style in the repository
- Do not introduce new dependencies without explicit issue approval

## File Ownership
| File | Owner | Other Agents |
|------|-------|--------------|
| `goal.md` | Human | Read-only |
| `memory.md` | Manager | Do not touch |
| `run.sh` | Human | Do not touch |
| `run.py` | Human | Do not touch |
| `opencode.json` | Human | Do not touch |
| `AGENTS.md` | Human | Do not touch |
| `.opencode/agents/reviewer.md` | Human | Do not touch |
| Source code | SWE | Manager and Reviewer read only |
