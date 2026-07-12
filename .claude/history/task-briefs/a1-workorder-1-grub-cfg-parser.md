# WORK-ORDER A1/1 — grub.cfg menu-entry parser (for the local Builder model)

> Uwaga dla właściciela: ten plik jest samowystarczalnym promptem dla
> lokalnego modelu. Wklej jego treść (od linii "ROLE" w dół) do LM Studio /
> Cline. Model ma edytować **wyłącznie** `crates/core/src/grub_cfg.rs`.

---

ROLE: You are a careful Rust engineer implementing ONE function in an
existing codebase. You write production-grade parser code.

TASK: Implement `parse_menu_entries` in the file
`crates/core/src/grub_cfg.rs`, replacing the `todo!()` body. The file
already contains: full module docs, the `GrubMenuEntry` struct, the
function signature with a doctest, and 15 unit tests that define the
exact contract.

DO:
- Edit ONLY `crates/core/src/grub_cfg.rs`. Do not touch any other file.
- Rename the parameter `_cfg` to `cfg` when you implement the body.
- Make ALL 15 tests in the file pass: run
  `cargo test -p bootcontrol-core grub_cfg`
  and iterate until: `15 passed; 0 failed`.
- Also make the doctest pass: `cargo test -p bootcontrol-core --doc`.
- You may add private helper functions and private `#[cfg(test)]` helpers
  BELOW the public function, but the public API (struct + function
  signature) must stay exactly as-is.

HARD RULES (this repo enforces them in CI):
- NO `unwrap()`, NO `expect()`, NO `panic!()`, NO `unreachable!()` in
  production code (tests may use them). Return
  `BootControlError::MalformedValue { key: "grub.cfg".into(), reason: … }`
  for every malformed-input case.
- NO I/O: no `std::fs`, no `std::env`, no `std::process`. The function is
  pure: `&str` in, `Result` out. It must never panic on ANY input,
  including garbage bytes, huge lines, or deeply nested braces.
- NO new dependencies, NO regex crate — plain `std` string processing.
- `cargo clippy -p bootcontrol-core -- -D warnings` must be clean.
- Keep comment density similar to the rest of the file; comment only
  non-obvious invariants (e.g. the quote-escape state machine).

PARSING SPEC (the tests are the authority when in doubt):
1. Scan the input line by line. Track a stack of open blocks.
2. A line whose first word (after optional leading whitespace) is
   `menuentry` or `submenu` — and which is NOT inside a menuentry body —
   starts a menu item. Everything else (comments starting with `#`,
   `set`, `if`/`fi`, `function` blocks, body lines) is ignored, but `{`
   / `}` braces of non-menu blocks (e.g. `function load_video {`) still
   need tracking so their closing `}` is not mistaken for a menu item's.
   Simplification allowed: you only need brace tracking good enough for
   the shapes in the tests (function blocks at top level, entry bodies,
   nested submenus).
3. The item's TITLE is the first quoted word after the keyword:
   - single quotes: no escapes inside; the generated-by-grub-mkconfig
     apostrophe idiom `'\''` (close quote + escaped apostrophe + reopen)
     must decode to a literal `'` — see test
     `apostrophe_escape_in_single_quoted_title`.
   - double quotes: take content verbatim up to the closing `"`.
   - unterminated quote ⇒ MalformedValue.
4. The item's ID is the quoted word right after the literal token
   `$menuentry_id_option`, when present; otherwise `None`.
5. The menuentry/submenu line MUST contain the opening `{` on the same
   line — otherwise MalformedValue (see `missing_opening_brace_is_malformed`).
6. Numbering: at each nesting level, items (menuentry AND submenu alike)
   are numbered 0,1,2,… in file order. `path` is the `>`-joined chain of
   indices from top level down, e.g. `"1>0"`. `depth` = number of
   ancestors. `is_submenu` = the keyword was `submenu`.
7. A `menuentry` body may contain arbitrary lines including braces in
   commands; count unquoted `{`/`}` to find the body's end. Lines inside
   a menuentry body never start new items (see `body_lines_never_produce_entries`).
   A `submenu` body DOES contain nested items — recurse/continue scanning
   inside it.
8. EOF with any block still open ⇒ MalformedValue.
9. Empty input or input with no menu items ⇒ `Ok(vec![])`.

OUTPUT FORMAT: reply with the complete final content of
`crates/core/src/grub_cfg.rs` (or apply the edit directly if you have
file access). Then state the test command you expect to pass.

ACCEPTANCE (the human reviewer will check):
- `cargo test -p bootcontrol-core grub_cfg` → 15 passed
- `cargo test -p bootcontrol-core --doc` → green
- `cargo clippy -p bootcontrol-core -- -D warnings` → clean
- no changes outside `crates/core/src/grub_cfg.rs`
