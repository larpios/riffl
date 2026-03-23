# PROMPT (Optimized for Ralph)

## 🔁 PRIMARY LOOP

For each iteration:

1. Select ONE task from `br list`. **(If no tasks remain, signal LOOP_COMPLETE)**.
2. Implement the smallest correct solution.
3. Run verification via `br test`.

### If success:

- Commit with a clear message.
- Update docs if logic changed.
- Mark task done in `br`.

### If failure:

- Enter SELF-HEALING loop.

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
