# PROMPT

## 🔁 PRIMARY LOOP (SELF-HEALING)

For each iteration:

1. Select ONE task from `br list`
2. Implement the smallest correct solution
3. Run verification (build/tests/reasoning)

### If success:

- Commit
- Update docs if needed
- Mark task done

### If failure:

- Enter SELF-HEALING loop

---

## 🩺 SELF-HEALING LOOP

When a build/test fails:

### Step 1 — Diagnose

Identify failure type:

- Compile error
- Runtime bug
- Logic error
- Missing context / unclear spec

Write a short diagnosis.

---

### Step 2 — Retry (max 3 times)

Each retry MUST change strategy:

1st retry → fix directly  
2nd retry → simplify approach  
3rd retry → isolate or reduce scope

DO NOT repeat the same attempt.

---

### Step 3 — Decide

If resolved:
→ continue normally

If still failing after retries:
→ mark task as BLOCKED in `br`
→ explain why clearly
→ pick next task

---

## 🧠 FAILURE STRATEGIES

Use these patterns:

- **Simplify:** remove abstractions, make it dumb but correct
- **Isolate:** reduce to minimal reproducible case
- **Fallback:** implement partial or degraded behavior

Avoid:

- large rewrites
- guessing without evidence
- infinite retries

---

## 🧾 FAILURE MEMORY

Before retrying:

- Check `.ralph/scratchpad.md`
- Avoid repeating known failed approaches
- Record new insights after each failure

---

## 🎯 PRIORITIZATION

1. Fix broken core functionality first
2. Then unblock blocked tasks
3. Then implement features

---

## 🧪 DEFINITION OF DONE

- Builds successfully
- Behavior is correct or acceptable
- No obvious regressions
- Docs updated if needed

---

## 🧨 BLOCKING RULE

A task is BLOCKED if:

- Requires missing architecture decision
- Requires unclear spec
- Fails after 3 meaningful retries

When blocked:

- Log reason in `br`
- Suggest next action

---

## 🏁 LOOP TERMINATION

Signal LOOP_COMPLETE only if:

- No meaningful tasks remain
- OR all remaining tasks are blocked
