# AI-Assisted Development Workflow

This document describes how Repowatch is built using AI-assisted development. The goal is transparency and to help others learn from (and improve upon) this approach.

---

## Overview

Repowatch is 100% AI-generated code. All Rust code, documentation, and configuration is written by AI assistants via [Cursor](https://cursor.com) with MCP tools. The human developer's role is:

- **Product direction** — Deciding what to build and why
- **Testing** — Running the app, finding bugs, verifying features work
- **Feedback** — Describing problems when things don't work
- **Orchestration** — Managing the AI workflow, choosing models, reviewing handovers

---

## The Workflow

### Phase 1: Ideation

Before writing any code, discuss ideas with multiple AI assistants to get diverse perspectives:

| AI | Role |
|----|------|
| **Claude** | Architecture decisions, Rust-specific guidance |
| **Perplexity** | Research on libraries, best practices |
| **Gemini** | Alternative perspectives, code review |

### Phase 2: PRD Creation

Requirements are captured in a **Product Requirements Document** (`prd.txt`). This defines:

- Problem statement and target users
- Feature specifications with acceptance criteria
- Technical architecture and crate choices
- Data models and API design

The PRD is the source of truth for what we're building.

### Phase 3: Task Generation

The PRD is parsed by [Task Master](https://github.com/task-master-ai/task-master), an AI-powered task management tool. It:

1. Reads the PRD
2. Generates structured tasks with dependencies
3. Provides **complexity analysis** (1–9 scale) for each task
4. Breaks down complex tasks into subtasks

The result is a `tasks.json` file with actionable implementation steps. Complexity scores drive model selection (see below).

### Phase 4: Model Selection by Complexity

This is a key difference from earlier workflows. Instead of using one model for everything, we use Task Master's complexity analysis to choose the right model for each task:

| Complexity | Model | Rationale |
|------------|-------|-----------|
| **1–7** | Kimi 2.5k | Standard tasks — API calls, widgets, cache logic, tests. Fast and cost-effective. |
| **7–9** | Opus 4.6 | Complex architecture, multi-module refactors, subtle bugs. Worth the slower speed. |

**Why this matters:**
- Lower-complexity tasks don't need the most expensive model
- Higher-complexity tasks benefit from deeper reasoning
- When in doubt, start with Kimi 2.5k and escalate if the task stalls

### Phase 5: Implementation

Each task gets a **fresh chat session** in Cursor. The workflow:

1. **Attach context** — `@docs/ai-context.md` + `@docs/index.md` (always) + paste `current-handover-prompt.md` content
2. **AI reads task** — Understands requirements from handover + PRD context
3. **AI explores codebase** — Finds relevant files, understands patterns
4. **AI implements** — Writes code following project conventions from `ai-context.md`
5. **AI verifies** — Runs `cargo build`, checks for errors
6. **AI documents** — Creates feature docs in `docs/technical/`, updates `docs/index.md`

### Phase 6: Session Handover

AI assistants don't have persistent memory between sessions. Context is maintained through handover documents.

**The cycle:**

```
┌─────────────────────────────────────────────────────┐
│  1. START NEW CHAT                                  │
│     - Attach @docs/ai-context.md + @docs/index.md  │
│     - Paste current-handover-prompt.md content      │
│     - AI reads task and begins work                 │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│  2. WORK ON TASK                                    │
│     - AI implements the task                        │
│     - Human reviews, tests, provides feedback       │
│     - Iterate until task is complete                │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│  3. COMPLETE TASK                                   │
│     - Paste update-handover-prompt.md into chat     │
│     - AI marks task done in Task Master             │
│     - AI creates feature documentation              │
│     - AI updates current-handover-prompt for next   │
│     - Human reviews handover, picks model for next  │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│  4. CLOSE CHAT, START FRESH                         │
│     - Close current chat (context is now stale)     │
│     - Check next task complexity → pick model       │
│     - Go to step 1 with new chat                    │
└─────────────────────────────────────────────────────┘
```

**Why fresh chats?** AI context accumulates noise over a session. Starting fresh ensures the AI only has the clean, relevant context from the handover.

### Phase 7: Human Review

Every session ends with human review:

1. **Test the changes** — Run the app, verify the feature works
2. **Check for regressions** — Make sure existing features still work
3. **Evaluate the result** — Does it match what was requested?
4. **Commit or iterate** — Accept changes or request fixes

Focus on **functional testing** — does it work? Does it break anything? The AI handles code quality; you handle product quality.

---

## File Structure

The workflow is driven by a small set of files:

| File | Purpose | Attached to Chat? |
|------|---------|-------------------|
| `docs/ai-context.md` | Project rules, architecture, conventions | Always |
| `docs/index.md` | Documentation index — helps AI find relevant docs | Always |
| `docs/current-handover-prompt.md` | Current task details, key files | Pasted at start |
| `docs/update-handover-prompt.md` | Instructions for post-task handover update | Pasted at end |
| `prd.txt` | Product Requirements Document | On demand |

### What Goes Where

A key design decision: **rules live in `ai-context.md`, not in the handover**.

| Content | Location | Rationale |
|---------|----------|-----------|
| Project rules | `ai-context.md` | Persistent, same for every task |
| Architecture & conventions | `ai-context.md` | Stable across sessions |
| Model selection guide | `ai-context.md` | Referenced every session |
| Current task details | `current-handover-prompt.md` | Changes every task |
| Key files for task | `current-handover-prompt.md` | Changes every task |
| Task-specific context | `current-handover-prompt.md` | Changes every task |

This separation means:
- **`ai-context.md`** answers "how should I write code for this project?"
- **`current-handover-prompt.md`** answers "what should I work on right now?"

### Constraints

| File | Max Lines | Rule |
|------|-----------|------|
| `ai-context.md` | ~100 | No history, no roadmap, no task-specific content |
| `index.md` | Grows naturally | Pure index — no architecture, no code examples |
| `current-handover-prompt.md` | As short as possible | Only current task context, nothing from previous tasks |

---

## Documentation Requirements

Every completed task should produce feature-based documentation.

**Naming:**
- Good: `github-api-client.md`, `cache-layer.md`, `tui-panels.md`
- Bad: `task-1.md`, `subtask-2-3.md`

**Where:**
- `docs/technical/` — Implementation details
- `docs/guides/` — User-facing guides

**Always update `docs/index.md`** — This is how the AI finds relevant docs in future sessions. A well-maintained index saves context and improves AI accuracy.

---

## Tools Used

| Tool | Purpose |
|------|---------|
| [Cursor](https://cursor.com) | AI-powered IDE with Claude/model integration |
| [Task Master](https://github.com/task-master-ai/task-master) | AI task management, PRD parsing, complexity analysis |
| [Context7](https://context7.com) | MCP tool for fetching up-to-date library documentation |
| Kimi 2.5k | Primary coding model for complexity 1–7 tasks |
| Opus 4.6 | Advanced model for complexity 7–9 tasks |
| Git | Version control |

---

## Differences from Ferrite Workflow

This workflow is evolved from the [Ferrite AI development workflow](https://github.com/OlaProeis/Ferrite/blob/master/docs/ai-workflow/ai-development-workflow.md). Key changes:

| Aspect | Ferrite | Repowatch |
|--------|---------|-----------|
| **Model selection** | One model, escalate manually | Complexity-driven: 1–7 Kimi 2.5k, 7–9 Opus 4.6 |
| **Project rules** | In `current-handover-prompt.md` | In `ai-context.md` (persistent, not copied per task) |
| **Handover content** | Rules + environment + task | Environment + task only (leaner) |
| **Context attachment** | `@ai-context.md` optional | `@ai-context.md` + `@index.md` always attached |

### Why These Changes?

**Complexity-driven model selection** — Not every task needs the most powerful (and slowest/expensive) model. Task Master already analyzes complexity; we use that signal to pick the right tool for the job.

**Rules in ai-context instead of handover** — Rules don't change between tasks. Putting them in the handover meant copying them every time and risking drift. Keeping them in `ai-context.md` with a `(DO NOT UPDATE)` marker ensures consistency.

**Leaner handovers** — With rules removed, the handover is purely about the current task. Less noise, faster for the AI to parse, less chance of stale information.

---

## FAQ

### Why fresh chats for each task?

AI context accumulates noise — previous task details, dead-end explorations, resolved bugs. Starting fresh with a clean handover gives the AI exactly what it needs and nothing it doesn't.

### How do you handle tasks that span multiple sessions?

If a task isn't finished in one session, update the handover with progress notes and continue in a fresh chat. The handover carries the context forward.

### What if the complexity score seems wrong?

Start with the suggested model. If a "complexity 5" task turns out to be harder than expected, close the chat and restart with a more capable model. The handover makes this cheap.

### Could someone replicate this workflow?

Yes — that's why this document exists. The workflow is:

1. Write a clear PRD
2. Use Task Master to generate tasks with complexity analysis
3. Pick model based on complexity
4. Use Cursor with `ai-context.md` + handover for each task
5. Review, test, update handover, repeat
