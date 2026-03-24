# PROMPT (Optimized for Ralph)

## 🔁 PRIMARY LOOP

For each iteration:

### Phase 0 — Scan (when br ready is empty)

If `br ready` returns no tasks or all are blocked:

1. **Compile scan:** Run `cargo check` and capture output
2. **Lint scan:** Run `cargo clippy` and capture output
3. **Debt scan:** Search for `TODO`, `FIXME`, `XXX` patterns
4. **Gap scan:** Check for missing tests, docs, error handling
5. **Dedup:** Run `br search <keywords>` to avoid duplicates
6. **Create:** Create max 3 issues with `[auto]` prefix
7. **Select:** Pick highest priority and proceed to implement
8. **Complete:** If nothing actionable found, signal `LOOP_COMPLETE`

### Phase 1 — Select

1. Select ONE task from `br list`.
2. Implement the smallest correct solution.
3. Run verification via appropriate test suite.

### If success:

- Commit with a clear message.
- Update docs if logic changed.
- Mark task done in `br`.

### If failure:

- Enter SELF-HEALING loop.

---

## 📋 ISSUE CREATION RULES

When auto-scanning creates issues:

- **Prefix:** Always use `[auto]` to distinguish from human issues
- **Max rate:** Create max 3 issues per scan cycle (prevents overwhelming)
- **Dedup:** Always run `br search <keywords>` before creating to avoid duplicates
- **Content:** Include scan command output in issue body
- **Priority:** Mark as `ready` so Scouter can pick it up

---

## 🩺 SELF-HEALING LOOP

When a build/test fails:

### Step 1 — Diagnose

Identify failure: Compile, Runtime, Logic, or Missing Context.
**CRITICAL:** Write this diagnosis and the current failed code snippet to `.ralph/scratchpad/`.

### Step 2 — Retry (Strict Strategy Rotation)

Check `.ralph/scratchpad/` for previous attempts. You MUST shift strategy based on the attempt count:

- **1st retry:** Fix directly (logical correction).
- **2nd retry:** Simplify (remove complexity, use primitives).
- **3rd retry:** Isolate (create a standalone `repro.rs` or minimal test case).

**DO NOT** repeat an approach documented in the scratchpad.

### Step 3 — Decide

If still failing after 3 attempts:

- Mark task as **BLOCKED** in `br`.
- Append the "Final Blocker Reason" to the task description.
- Pick next task.

---

## 🧪 DEFINITION OF DONE

- `br test` passes $100\%$.
- No regressions in core functionality.
- Task status updated in `br`.
