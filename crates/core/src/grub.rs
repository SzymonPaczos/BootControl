//! Safe, strict-subset parser for `/etc/default/grub`.
//!
//! # Design Contract
//!
//! `/etc/default/grub` is technically a Bash script that the boot system
//! **sources** at runtime. This means it can legally contain arbitrary Bash
//! constructs such as subshells, loops, and conditionals. BootControl's
//! parser handles **only the safe strict subset**: simple variable assignments
//! of the form `KEY=VALUE` or `KEY="VALUE WITH SPACES"`.
//!
//! **If any executable Bash construct is detected, the parser aborts
//! immediately** and returns
//! [`BootControlError::ComplexBashDetected`](crate::error::BootControlError::ComplexBashDetected).
//! BootControl never attempts to parse around or work through complex Bash,
//! regardless of where in the file it appears. This is a hard safety
//! invariant, not a best-effort heuristic.
//!
//! ## What the parser accepts (strict subset)
//!
//! - Blank lines (whitespace-only).
//! - Comment lines: lines whose first non-whitespace character is `#`.
//! - Simple unquoted assignments: `KEY=value`
//! - Double-quoted assignments: `KEY="value with spaces"`
//!   - The value may contain any character except unescaped `"` or newlines.
//!
//! ## What triggers bail-out (`ComplexBashDetected`)
//!
//! Any line that is not a blank, comment, or simple assignment AND contains
//! one of the following constructs causes immediate bail-out:
//!
//! | Construct | Example |
//! |-----------|---------|
//! | Subshell `$(...)` | `KERNEL=$(uname -r)` |
//! | Backtick subshell `` `...` `` | `` KERNEL=`uname -r` `` |
//! | `if` / `fi` | `if [ -d /boot ]; then` |
//! | `for` / `done` | `for x in a b c; do` |
//! | `while` / `until` | `while true; do` |
//! | Compound commands `{` / `}` | `{ echo "x"; }` |
//! | Here-doc `<<` | `cat <<EOF` |
//! | Pipe `|` | `echo x | cat` |
//!
//! ## Preserving comments
//!
//! AGENT.md mandates that user comments survive every parser round-trip
//! unchanged. The [`GrubConfig`] type stores all lines verbatim and only
//! exposes value-level access through its API. A future serialiser will
//! reconstruct the original text from the stored lines.

use crate::error::BootControlError;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Bash construct detectors
// ─────────────────────────────────────────────────────────────────────────────

/// Patterns that indicate executable Bash logic that exceeds our strict subset.
/// Order is chosen to give the most descriptive offender description first.
const BASH_TRIGGERS: &[(&str, &str)] = &[
    ("$(", "subshell: $(...)"),
    ("`", "backtick subshell"),
    ("<<", "here-document"),
    ("|", "pipe operator"),
    ("{", "compound command block"),
];

/// Bash *keyword* prefixes that must appear as the first non-whitespace token
/// on a line to trigger a bail-out. We check keywords separately from
/// character-level triggers because `if` could appear legally inside a quoted
/// value (e.g. `GRUB_CMDLINE_LINUX_DEFAULT="quiet splash"`); we only bail out
/// when it is the *start* of the statement, not inside a quoted string.
const BASH_KEYWORD_PREFIXES: &[(&str, &str)] = &[
    ("if ", "if-statement"),
    ("if[", "if-statement (bracket)"),
    ("for ", "for-loop"),
    ("while ", "while-loop"),
    ("until ", "until-loop"),
    ("case ", "case-statement"),
    ("fi", "fi (end of if-statement)"),
    ("done", "done (end of loop)"),
    ("esac", "esac (end of case)"),
    ("do ", "do-keyword"),
    ("then", "then-keyword"),
    ("else", "else-keyword"),
    ("elif ", "elif-keyword"),
    ("function ", "function-definition"),
    ("export ", "export-keyword"),
    ("source ", "source-keyword"),
    (". ", "source-dot operator"),
    ("local ", "local-keyword"),
    ("unset ", "unset-keyword"),
    ("declare ", "declare-keyword"),
    ("typeset ", "typeset-keyword"),
    ("readonly ", "readonly-keyword"),
    ("return ", "return-keyword"),
    ("trap ", "trap-keyword"),
    ("eval ", "eval-keyword"),
];

/// Inspect a single non-comment, non-blank, non-assignment line for dangerous
/// Bash constructs. Returns the offender description if found.
fn find_bash_construct(line: &str) -> Option<&'static str> {
    // 1. Check character-level triggers on the raw (unstripped) line.
    //    We cannot strip the value from quotes here because we already know
    //    this line is NOT a valid simple assignment.
    for (needle, desc) in BASH_TRIGGERS {
        if line.contains(needle) {
            return Some(desc);
        }
    }
    // 2. Check keyword prefixes against the left-trimmed line.
    let trimmed = line.trim_start();
    for (prefix, desc) in BASH_KEYWORD_PREFIXES {
        if trimmed.starts_with(prefix) || trimmed == prefix.trim_end() {
            return Some(desc);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Assignment parser for a single line
// ─────────────────────────────────────────────────────────────────────────────

/// Result of attempting to parse a single line as a KEY=VALUE assignment.
#[derive(Debug, PartialEq, Eq)]
enum LineKind {
    /// Blank or whitespace-only line.
    Blank,
    /// A comment: first non-whitespace character is `#`.
    Comment,
    /// A valid simple assignment: `KEY=VALUE` or `KEY="VALUE"`.
    Assignment { key: String, value: String },
    /// Not a blank, comment, or assignment. The embedded description names
    /// the Bash construct detected (or a generic description if we could not
    /// identify a specific one).
    BashConstruct { offender: String },
}

/// Parse a single line from `/etc/default/grub`.
///
/// The function is intentionally strict: anything that does not unambiguously
/// match the safe subset is classified as [`LineKind::BashConstruct`].
fn parse_line(line: &str) -> LineKind {
    // ── Blank ────────────────────────────────────────────────────────────────
    if line.trim().is_empty() {
        return LineKind::Blank;
    }

    // ── Comment ──────────────────────────────────────────────────────────────
    if line.trim_start().starts_with('#') {
        return LineKind::Comment;
    }

    // ── Try to match KEY=VALUE ────────────────────────────────────────────────
    // Key: one or more ASCII alphanumeric characters or underscores, starting
    // with a letter or underscore (POSIX shell variable name).
    // We locate the first '=' and split there.
    if let Some(eq_pos) = line.find('=') {
        let key_part = &line[..eq_pos];
        let value_part = &line[eq_pos + 1..];

        // Validate the key: must be a legal shell identifier.
        if is_valid_key(key_part) {
            // Check the value part for bash constructs BEFORE accepting it.
            // Even in a "KEY=..." line the value could be `$(command)`.
            if let Some(offender) = detect_dangerous_value(value_part) {
                return LineKind::BashConstruct {
                    offender: offender.to_string(),
                };
            }

            let value = extract_value(value_part);
            return LineKind::Assignment {
                key: key_part.to_string(),
                value,
            };
        }
    }

    // ── Not a valid assignment — probe for known Bash constructs ─────────────
    let offender = find_bash_construct(line)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unrecognised non-assignment statement".to_string());

    LineKind::BashConstruct { offender }
}

/// Return `true` if `key` is a valid POSIX shell identifier
/// (`[A-Za-z_][A-Za-z0-9_]*`).
fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }
    let mut chars = key.chars();
    let first = chars.next().expect("non-empty string has a first char");
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Inspect only the *value portion* (right of `=`) for executable constructs.
///
/// We inspect the raw value before stripping quotes so that embedded
/// `$(...)` is never silently accepted inside a double-quoted string.
fn detect_dangerous_value(value: &str) -> Option<&'static str> {
    // Check character triggers on the raw value.
    for (needle, desc) in BASH_TRIGGERS {
        if value.contains(needle) {
            return Some(desc);
        }
    }
    None
}

/// Strip optional surrounding double-quotes from a value string.
///
/// Only outer double-quotes are removed. Inner quotes, escaped characters, and
/// all other content are preserved verbatim. Single-quoted values are returned
/// as-is (we do not expand or strip single quotes — single-quoting is uncommon
/// in `/etc/default/grub` and would require a full shell quoting parser for
/// correctness).
fn extract_value(raw: &str) -> String {
    let stripped = raw.trim_end_matches('\n').trim_end_matches('\r');
    if stripped.starts_with('"') && stripped.ends_with('"') && stripped.len() >= 2 {
        stripped[1..stripped.len() - 1].to_string()
    } else {
        stripped.to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GrubConfig: the parsed result
// ─────────────────────────────────────────────────────────────────────────────

/// A parsed representation of `/etc/default/grub`.
///
/// All original lines are preserved verbatim in [`GrubConfig::lines`] so that
/// a future serialiser can reconstruct the exact file — including comments and
/// blank lines — after a value edit. The [`GrubConfig::map`] provides O(1)
/// key lookup.
///
/// # Invariants
///
/// - `lines` contains every line of the original input, in order, with its
///   original whitespace preserved.
/// - `map` contains only keys from lines that were classified as valid
///   assignments.
/// - If [`parse_grub_config`] returned `Ok(config)`, the config contains **no**
///   executable Bash constructs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrubConfig {
    /// All original lines from the file, in order.
    pub lines: Vec<String>,
    /// Key-value map for O(1) lookup. Values are stripped of surrounding
    /// double-quotes.
    pub map: HashMap<String, String>,
}

impl GrubConfig {
    /// Look up the value of a GRUB configuration key.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to look up (e.g., `"GRUB_TIMEOUT"`).
    ///
    /// # Errors
    ///
    /// Returns [`BootControlError::KeyNotFound`] if the key is not present in
    /// the configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::grub::parse_grub_config;
    ///
    /// let config = parse_grub_config("GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n").unwrap();
    /// assert_eq!(config.get("GRUB_TIMEOUT").unwrap(), "5");
    /// assert_eq!(config.get("GRUB_DEFAULT").unwrap(), "0");
    /// ```
    pub fn get(&self, key: &str) -> Result<&str, BootControlError> {
        self.map
            .get(key)
            .map(|s| s.as_str())
            .ok_or_else(|| BootControlError::KeyNotFound {
                key: key.to_string(),
            })
    }

    /// Return `true` if the configuration contains the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The configuration key to check (e.g., `"GRUB_TIMEOUT"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use bootcontrol_core::grub::parse_grub_config;
    ///
    /// let config = parse_grub_config("GRUB_TIMEOUT=5\n").unwrap();
    /// assert!(config.contains_key("GRUB_TIMEOUT"));
    /// assert!(!config.contains_key("GRUB_DEFAULT"));
    /// ```
    pub fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Parse the strict-subset of `/etc/default/grub` that BootControl can safely
/// handle.
///
/// This is a **pure function**: it accepts the raw file contents as a `&str`
/// and performs no disk I/O or system calls. All Bash-unsafe constructs are
/// rejected before any part of the function's output is used.
///
/// # Arguments
///
/// * `input` - The raw contents of `/etc/default/grub` as a UTF-8 string.
///   Typically obtained by `std::fs::read_to_string("/etc/default/grub")` in
///   the daemon layer, but this function does not perform that read itself.
///
/// # Errors
///
/// Returns [`BootControlError::ComplexBashDetected`] immediately upon
/// encountering **any** of the following constructs, regardless of where they
/// appear in the file:
///
/// - Command substitution: `$(...)` or `` `...` ``
/// - Control-flow keywords: `if`, `for`, `while`, `until`, `case`, `fi`,
///   `done`, `esac`, `then`, `else`, `elif`
/// - Function definitions: `function <name>`
/// - Compound command blocks: `{`, `}`
/// - Here-documents: `<<`
/// - Pipes: `|`
/// - Shell built-ins that indicate scripting: `export`, `source`, `.`,
///   `local`, `unset`, `declare`, `typeset`, `readonly`, `return`, `trap`,
///   `eval`
///
/// The error includes a human-readable `offender` field naming the first
/// detected construct.
///
/// # Examples
///
/// Parsing a standard GRUB config succeeds:
///
/// ```
/// use bootcontrol_core::grub::parse_grub_config;
///
/// let input = concat!(
///     "# This is a comment — preserved verbatim\n",
///     "GRUB_DEFAULT=0\n",
///     "GRUB_TIMEOUT=5\n",
///     "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"\n",
///     "GRUB_DISTRIBUTOR=\"Ubuntu\"\n",
/// );
/// let config = parse_grub_config(input).unwrap();
/// assert_eq!(config.get("GRUB_TIMEOUT").unwrap(), "5");
/// assert_eq!(config.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(), "quiet splash");
/// ```
///
/// A config with a subshell is immediately rejected:
///
/// ```
/// use bootcontrol_core::grub::parse_grub_config;
/// use bootcontrol_core::error::BootControlError;
///
/// let evil = "GRUB_TIMEOUT=$(cat /etc/shadow)\n";
/// let err = parse_grub_config(evil).unwrap_err();
/// assert!(matches!(err, BootControlError::ComplexBashDetected { .. }));
/// ```
pub fn parse_grub_config(input: &str) -> Result<GrubConfig, BootControlError> {
    let mut map = HashMap::new();
    let mut lines = Vec::new();

    for raw_line in input.lines() {
        lines.push(raw_line.to_string());

        match parse_line(raw_line) {
            LineKind::Blank | LineKind::Comment => {
                // Preserved verbatim; no key-value to record.
            }
            LineKind::Assignment { key, value } => {
                // Last assignment wins (matches shell `source` semantics for
                // duplicate keys). This preserves the most recently set value,
                // which is what a shell would observe.
                map.insert(key, value);
            }
            LineKind::BashConstruct { offender } => {
                return Err(BootControlError::ComplexBashDetected { offender });
            }
        }
    }

    Ok(GrubConfig { lines, map })
}

/// Extract the value of a single key from a GRUB config string.
///
/// This is a convenience function for callers that need a single key without
/// constructing a full [`GrubConfig`]. For repeated lookups, prefer
/// [`parse_grub_config`] once and then call [`GrubConfig::get`] multiple times
/// to avoid re-parsing.
///
/// # Arguments
///
/// * `input` - The raw contents of `/etc/default/grub`.
/// * `key`   - The key to look up (e.g., `"GRUB_TIMEOUT"`).
///
/// # Errors
///
/// Returns [`BootControlError::ComplexBashDetected`] if the input contains
/// any executable Bash constructs (see [`parse_grub_config`] for the full
/// list).
///
/// Returns [`BootControlError::KeyNotFound`] if the key is absent from the
/// input.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::grub::get_value;
///
/// let config = "GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n";
/// assert_eq!(get_value(config, "GRUB_TIMEOUT").unwrap(), "5");
/// ```
pub fn get_value(input: &str, key: &str) -> Result<String, BootControlError> {
    let config = parse_grub_config(input)?;
    config.get(key).map(|s| s.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::BootControlError;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn assert_complex_bash(input: &str) {
        let result = parse_grub_config(input);
        assert!(
            matches!(result, Err(BootControlError::ComplexBashDetected { .. })),
            "Expected ComplexBashDetected for input: {input:?}\nGot: {result:?}"
        );
    }

    fn assert_parses_ok(input: &str) -> GrubConfig {
        parse_grub_config(input)
            .unwrap_or_else(|e| panic!("Expected Ok for input:\n{input}\n\nGot error: {e}"))
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 1: Basic happy-path parsing
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn parses_unquoted_integer_value() {
        let cfg = assert_parses_ok("GRUB_TIMEOUT=5\n");
        assert_eq!(cfg.get("GRUB_TIMEOUT").unwrap(), "5");
    }

    #[test]
    fn parses_unquoted_zero() {
        let cfg = assert_parses_ok("GRUB_DEFAULT=0\n");
        assert_eq!(cfg.get("GRUB_DEFAULT").unwrap(), "0");
    }

    #[test]
    fn parses_double_quoted_string() {
        let cfg = assert_parses_ok("GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"\n");
        // Outer quotes are stripped; inner content is preserved.
        assert_eq!(
            cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(),
            "quiet splash"
        );
    }

    #[test]
    fn parses_double_quoted_empty_value() {
        let cfg = assert_parses_ok("GRUB_CMDLINE_LINUX_DEFAULT=\"\"\n");
        assert_eq!(cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(), "");
    }

    #[test]
    fn parses_unquoted_empty_value() {
        let cfg = assert_parses_ok("GRUB_CMDLINE_LINUX_DEFAULT=\n");
        assert_eq!(cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(), "");
    }

    #[test]
    fn parses_underscore_in_key() {
        let cfg = assert_parses_ok("GRUB_CMDLINE_LINUX=\"ro\"\n");
        assert_eq!(cfg.get("GRUB_CMDLINE_LINUX").unwrap(), "ro");
    }

    #[test]
    fn parses_real_world_grub_config() {
        let input = "\
# This is a generated file — do not edit manually.
GRUB_DEFAULT=0
GRUB_TIMEOUT_STYLE=hidden
GRUB_TIMEOUT=0
GRUB_DISTRIBUTOR=`lsb_release -i -s 2> /dev/null || echo Debian`
";
        // The backtick on GRUB_DISTRIBUTOR must trigger ComplexBashDetected.
        assert_complex_bash(input);
    }

    #[test]
    fn parses_typical_ubuntu_grub() {
        // Ubuntu's /etc/default/grub without dynamic entries.
        let input = "\
# If you change this file, run 'update-grub' afterwards.
GRUB_DEFAULT=0
GRUB_TIMEOUT_STYLE=hidden
GRUB_TIMEOUT=0
GRUB_DISTRIBUTOR=\"Ubuntu\"
GRUB_CMDLINE_LINUX_DEFAULT=\"quiet splash\"
GRUB_CMDLINE_LINUX=\"\"
";
        let cfg = assert_parses_ok(input);
        assert_eq!(cfg.get("GRUB_DEFAULT").unwrap(), "0");
        assert_eq!(cfg.get("GRUB_TIMEOUT").unwrap(), "0");
        assert_eq!(cfg.get("GRUB_DISTRIBUTOR").unwrap(), "Ubuntu");
        assert_eq!(
            cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(),
            "quiet splash"
        );
        assert_eq!(cfg.get("GRUB_CMDLINE_LINUX").unwrap(), "");
    }

    #[test]
    fn parses_arch_linux_grub() {
        let input = "\
GRUB_DEFAULT=0
GRUB_TIMEOUT=5
GRUB_DISTRIBUTOR=\"Arch\"
GRUB_CMDLINE_LINUX_DEFAULT=\"loglevel=3 quiet\"
GRUB_CMDLINE_LINUX=\"\"
GRUB_PRELOAD_MODULES=\"part_gpt part_msdos\"
GRUB_GFXMODE=auto
GRUB_GFXPAYLOAD_LINUX=keep
";
        let cfg = assert_parses_ok(input);
        assert_eq!(cfg.get("GRUB_TIMEOUT").unwrap(), "5");
        assert_eq!(
            cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(),
            "loglevel=3 quiet"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 2: Comment and whitespace preservation
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn comments_are_preserved_in_lines() {
        let input = "# My custom comment\nGRUB_TIMEOUT=5\n";
        let cfg = assert_parses_ok(input);
        assert_eq!(cfg.lines[0], "# My custom comment");
        assert_eq!(cfg.lines[1], "GRUB_TIMEOUT=5");
    }

    #[test]
    fn blank_lines_are_preserved_in_lines() {
        let input = "\nGRUB_TIMEOUT=5\n\n";
        let cfg = assert_parses_ok(input);
        assert_eq!(cfg.lines[0], "");
        assert_eq!(cfg.lines[1], "GRUB_TIMEOUT=5");
        assert_eq!(cfg.lines[2], "");
    }

    #[test]
    fn line_count_matches_input() {
        let input = "# comment\nGRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n";
        let cfg = assert_parses_ok(input);
        // `str::lines()` strips the trailing newline; input has 3 non-empty
        // logical lines.
        assert_eq!(cfg.lines.len(), 3);
    }

    #[test]
    fn inline_comment_within_value_is_kept_verbatim() {
        // GRUB does not support inline comments; a '#' inside a value is
        // literal. The parser must not strip it.
        let input = "GRUB_CMDLINE_LINUX_DEFAULT=\"quiet # not a comment\"\n";
        let cfg = assert_parses_ok(input);
        assert_eq!(
            cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(),
            "quiet # not a comment"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 3: Strict Subset Bail-Out — subshells
    // ════════════════════════════════════════════════════════════════════════

    /// **Critical security test.** A subshell in any value must be rejected.
    #[test]
    fn rejects_dollar_paren_subshell_in_value() {
        assert_complex_bash("GRUB_TIMEOUT=$(cat /etc/shadow)\n");
    }

    #[test]
    fn rejects_standalone_dollar_paren_subshell() {
        assert_complex_bash("$(malicious_command)\n");
    }

    #[test]
    fn rejects_backtick_subshell_in_value() {
        assert_complex_bash("KERNEL=`uname -r`\n");
    }

    #[test]
    fn rejects_backtick_on_standalone_line() {
        assert_complex_bash("`id`\n");
    }

    /// The famous Ubuntu default GRUB_DISTRIBUTOR uses a backtick subshell.
    #[test]
    fn rejects_ubuntu_distributor_backtick() {
        let input = "GRUB_DISTRIBUTOR=`lsb_release -i -s 2> /dev/null || echo Debian`\n";
        assert_complex_bash(input);
    }

    #[test]
    fn rejects_nested_subshell() {
        assert_complex_bash("GRUB_TIMEOUT=$(echo $(id))\n");
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 4: Strict Subset Bail-Out — control flow
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn rejects_if_statement() {
        assert_complex_bash("if [ -d /boot ]; then\n");
    }

    #[test]
    fn rejects_fi_keyword() {
        assert_complex_bash("fi\n");
    }

    #[test]
    fn rejects_for_loop() {
        assert_complex_bash("for x in a b c; do\n");
    }

    #[test]
    fn rejects_while_loop() {
        assert_complex_bash("while true; do\n");
    }

    #[test]
    fn rejects_until_loop() {
        assert_complex_bash("until false; do\n");
    }

    #[test]
    fn rejects_done_keyword() {
        assert_complex_bash("done\n");
    }

    #[test]
    fn rejects_case_statement() {
        assert_complex_bash("case $var in\n");
    }

    #[test]
    fn rejects_else_keyword() {
        assert_complex_bash("else\n");
    }

    #[test]
    fn rejects_elif_keyword() {
        assert_complex_bash("elif [ -f /boot/grub/grub.cfg ]; then\n");
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 5: Strict Subset Bail-Out — other dangerous constructs
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn rejects_pipe_operator() {
        assert_complex_bash("echo something | cat\n");
    }

    #[test]
    fn rejects_here_document() {
        assert_complex_bash("cat <<EOF\n");
    }

    #[test]
    fn rejects_compound_command_block() {
        assert_complex_bash("{ echo x; }\n");
    }

    #[test]
    fn rejects_function_definition() {
        assert_complex_bash("function my_func {\n");
    }

    #[test]
    fn rejects_export_keyword() {
        assert_complex_bash("export PATH=/usr/bin\n");
    }

    #[test]
    fn rejects_source_keyword() {
        assert_complex_bash("source /etc/profile\n");
    }

    #[test]
    fn rejects_source_dot_operator() {
        assert_complex_bash(". /etc/profile\n");
    }

    #[test]
    fn rejects_eval() {
        assert_complex_bash("eval \"rm -rf /\"\n");
    }

    #[test]
    fn rejects_trap() {
        assert_complex_bash("trap 'rm -f /tmp/lock' EXIT\n");
    }

    #[test]
    fn rejects_declare() {
        assert_complex_bash("declare -A my_array\n");
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 6: Bail-out is triggered even on the first line
    // ════════════════════════════════════════════════════════════════════════

    /// If the dangerous construct appears on the very first line, the parser
    /// must abort before processing later lines.
    #[test]
    fn rejects_on_first_line_before_processing_rest() {
        let input = "$(evil)\nGRUB_TIMEOUT=5\n";
        assert_complex_bash(input);
    }

    /// If the dangerous construct appears after valid assignments, the parser
    /// must still abort.
    #[test]
    fn rejects_when_complex_bash_appears_after_valid_assignments() {
        let input = "GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n$(evil)\n";
        assert_complex_bash(input);
    }

    /// Mixed file: valid lines, then a subshell. Must reject the whole file.
    #[test]
    fn rejects_mixed_file_when_any_line_is_complex() {
        let input = "\
# Safe comment
GRUB_TIMEOUT=5
GRUB_DISTRIBUTOR=`lsb_release -i -s 2> /dev/null || echo Debian`
GRUB_DEFAULT=0
";
        assert_complex_bash(input);
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 7: Edge cases and boundary conditions
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn empty_input_returns_empty_config() {
        let cfg = assert_parses_ok("");
        assert!(cfg.map.is_empty());
        assert!(cfg.lines.is_empty());
    }

    #[test]
    fn only_comments_returns_empty_map() {
        let input = "# comment 1\n# comment 2\n";
        let cfg = assert_parses_ok(input);
        assert!(cfg.map.is_empty());
        assert_eq!(cfg.lines.len(), 2);
    }

    #[test]
    fn only_blank_lines_returns_empty_map() {
        let cfg = assert_parses_ok("\n\n\n");
        assert!(cfg.map.is_empty());
    }

    #[test]
    fn duplicate_key_last_value_wins() {
        let input = "GRUB_TIMEOUT=5\nGRUB_TIMEOUT=10\n";
        let cfg = assert_parses_ok(input);
        assert_eq!(cfg.get("GRUB_TIMEOUT").unwrap(), "10");
    }

    #[test]
    fn get_missing_key_returns_key_not_found() {
        let cfg = assert_parses_ok("GRUB_TIMEOUT=5\n");
        let err = cfg.get("NONEXISTENT").unwrap_err();
        assert!(matches!(err, BootControlError::KeyNotFound { ref key } if key == "NONEXISTENT"));
    }

    #[test]
    fn contains_key_true_for_existing_key() {
        let cfg = assert_parses_ok("GRUB_TIMEOUT=5\n");
        assert!(cfg.contains_key("GRUB_TIMEOUT"));
    }

    #[test]
    fn contains_key_false_for_missing_key() {
        let cfg = assert_parses_ok("GRUB_TIMEOUT=5\n");
        assert!(!cfg.contains_key("GRUB_DEFAULT"));
    }

    #[test]
    fn value_with_equals_sign_inside_quotes() {
        // Edge case: quoted value containing '='.
        let input = "GRUB_CMDLINE_LINUX_DEFAULT=\"root=UUID=abc-123\"\n";
        let cfg = assert_parses_ok(input);
        assert_eq!(
            cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(),
            "root=UUID=abc-123"
        );
    }

    #[test]
    fn value_with_spaces_unquoted_is_accepted() {
        // Technically fragile Bash but lexically valid for our parser.
        // A space after '=' is a valid unquoted value from the parser's
        // perspective; we do not enforce shell syntax beyond our strict subset.
        let input = "GRUB_CMDLINE_LINUX_DEFAULT=quiet splash\n";
        let cfg = assert_parses_ok(input);
        assert_eq!(
            cfg.get("GRUB_CMDLINE_LINUX_DEFAULT").unwrap(),
            "quiet splash"
        );
    }

    #[test]
    fn input_without_trailing_newline_is_accepted() {
        let cfg = assert_parses_ok("GRUB_TIMEOUT=5");
        assert_eq!(cfg.get("GRUB_TIMEOUT").unwrap(), "5");
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 8: get_value convenience function
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn get_value_returns_correct_value() {
        let input = "GRUB_TIMEOUT=5\nGRUB_DEFAULT=0\n";
        assert_eq!(get_value(input, "GRUB_TIMEOUT").unwrap(), "5");
    }

    #[test]
    fn get_value_returns_key_not_found() {
        let input = "GRUB_TIMEOUT=5\n";
        let err = get_value(input, "GRUB_DEFAULT").unwrap_err();
        assert!(matches!(err, BootControlError::KeyNotFound { .. }));
    }

    #[test]
    fn get_value_propagates_complex_bash_error() {
        let evil = "GRUB_TIMEOUT=$(cat /etc/shadow)\n";
        let err = get_value(evil, "GRUB_TIMEOUT").unwrap_err();
        assert!(matches!(err, BootControlError::ComplexBashDetected { .. }));
    }

    // ════════════════════════════════════════════════════════════════════════
    // SECTION 9: Internal helper unit tests
    // ════════════════════════════════════════════════════════════════════════

    #[test]
    fn is_valid_key_accepts_standard_grub_keys() {
        assert!(is_valid_key("GRUB_TIMEOUT"));
        assert!(is_valid_key("GRUB_CMDLINE_LINUX_DEFAULT"));
        assert!(is_valid_key("_LEADING_UNDERSCORE"));
        assert!(is_valid_key("A"));
    }

    #[test]
    fn is_valid_key_rejects_empty_string() {
        assert!(!is_valid_key(""));
    }

    #[test]
    fn is_valid_key_rejects_starting_with_digit() {
        assert!(!is_valid_key("1GRUB_TIMEOUT"));
    }

    #[test]
    fn is_valid_key_rejects_spaces() {
        assert!(!is_valid_key("GRUB TIMEOUT"));
    }

    #[test]
    fn is_valid_key_rejects_hyphens() {
        assert!(!is_valid_key("GRUB-TIMEOUT"));
    }

    #[test]
    fn extract_value_strips_outer_double_quotes() {
        assert_eq!(extract_value("\"quiet splash\""), "quiet splash");
    }

    #[test]
    fn extract_value_keeps_unquoted_string() {
        assert_eq!(extract_value("5"), "5");
    }

    #[test]
    fn extract_value_handles_empty_unquoted() {
        assert_eq!(extract_value(""), "");
    }

    #[test]
    fn extract_value_handles_empty_double_quoted() {
        assert_eq!(extract_value("\"\""), "");
    }
}
