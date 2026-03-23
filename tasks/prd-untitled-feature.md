I need a bit more detail before I can write a solid PRD for this effect-command bug.

1. What behavior should `3xx` follow?
   A. ProTracker/XM semantics: `300` continues the previous tone portamento speed
   B. Strict explicit semantics: `300` means speed `0`, so no movement
   C. Compatibility mode: continue previous speed only if one was already set, otherwise no-op
   D. Other: specify exact tracker behavior to match

2. What scope do you want for this fix?
   A. Playback only: fix how the audio engine interprets `3xx`
   B. Playback + parser/editor display: interpretation and any UI/help text should match
   C. Playback + import/export compatibility for tracker formats
   D. Full effect-command audit beyond `3xx`

3. Which cases must be covered in the PRD?
   A. `300` continue only
   B. `300` continue + `3xx` with nonzero speed sets new speed
   C. B + interaction with `5xy` tone portamento + volume slide
   D. C + regression coverage for imported XM/S3M/IT tone portamento semantics

4. What is the desired compatibility target?
   A. XM/FastTracker-style behavior
   B. IT/Impulse Tracker-style behavior
   C. ProTracker-style behavior
   D. Match current project docs/tests first, even if not tracker-authentic

5. What quality gates must pass for each story?
   A. `cargo test --workspace --all-features`
   B. `cargo test --workspace --all-features` and `cargo clippy --workspace --all-features -- -D warnings`
   C. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features -- -D warnings`, and `cargo test --workspace --all-features`
   D. Other: specify exact commands

Reply in shorthand like `1A, 2B, 3C, 4A, 5C`. If you want, add one sentence describing the exact incorrect behavior you observed.