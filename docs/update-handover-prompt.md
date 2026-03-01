# Update Handover Instructions

Task is complete. Update the handover for the next session.

---

## 1. Mark Current Task Done

Use Task Master MCP tool to set status:

```
set_task_status --id=<current-task-id> --status=done
```

Prefer MCP tools over CLI commands for Task Master operations.

## 2. Create Documentation

Create feature-based documentation for completed work.

### Process

1. Identify what was implemented (group by feature, not task)
2. Create doc in `docs/technical/` or `docs/guides/`
3. **Update `docs/index.md`** with new entry

### Naming

- Good: `github-api-client.md`, `cache-layer.md`, `tui-panels.md`
- Bad: `task-1.md`, `subtask-2-3.md`

### Template

```markdown
# [Feature Name]

## Overview
Brief description of what was implemented.

## Key Files
- `src/path/to/file.rs` - Description

## Implementation Details
Key technical decisions and approach.

## Dependencies Used
- `crate_name` - Purpose

## Usage
How to use or test this feature.
```

## 3. Get Next Task

```
next_task
```

Or fetch a specific task:

```
get_task --id=<next-task-id>
```

## 4. Update current-handover-prompt.md

### Replace These Sections

| Section | New Content |
|---------|-------------|
| **Current Task** | Full details of next task (ID, title, description, complexity, implementation notes, test strategy) |
| **Key Files** | Only files relevant to the NEW task |
| **Context** | Only if needed for the new task |

### Keep These Sections (usually unchanged)

| Section | When to Update |
|---------|----------------|
| **Environment** | Only if version or tech stack changed |

### Remove

- Any previous task details
- Task-specific context that doesn't apply to new task

## 5. Update ai-context.md (if needed)

If the completed task changed the architecture significantly:

- Update the Architecture section
- Add new modules to "Where Things Will Live"
- Add new crate dependencies to the Crate Stack
- Keep it under ~100 lines

## 6. Verification Checklist

- [ ] Current task marked as `done` in Task Master
- [ ] Feature documentation created in `docs/technical/` or `docs/guides/`
- [ ] `docs/index.md` updated with new doc entry
- [ ] `current-handover-prompt.md` updated with next task
- [ ] Handover contains ONLY next task context
- [ ] `ai-context.md` updated if architecture changed
- [ ] Code compiles: `cargo build`
- [ ] `cargo clippy` passes clean

---

## Notes

- **Use MCP tools instead of CLI** — Prefer Task Master MCP tools (`set_task_status`, `next_task`, `get_task`) over CLI commands
- Keep the handover **minimal and focused** on the next task
- Don't include full task lists or project overviews
- Model selection for next task: check complexity in Task Master (1-7 → Kimi 2.5k, 7-9 → Opus 4.6)
