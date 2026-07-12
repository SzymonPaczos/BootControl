//! Parser for the *generated* `grub.cfg` menu structure.
//!
//! This module is the read-only half of GRUB menu-entry management
//! (A1 chunk 1, spec v1 §3.2 + v2 §10.2): it extracts the list of
//! `menuentry` / `submenu` items that GRUB would show at boot, so the
//! frontends can display them and `GRUB_DEFAULT` can be set by index
//! path or id.
//!
//! It never mutates anything — `grub.cfg` is a generated artifact owned
//! by `grub-mkconfig`. Mutation happens upstream (`/etc/default/grub`,
//! `/etc/grub.d/`) in later chunks.
//!
//! Like every parser in this crate it is a pure function over `&str`
//! (see `crates/core/CLAUDE.md`): no I/O, no `unwrap`/`expect`, no
//! panics on any input.

use crate::error::BootControlError;

/// One boot menu item parsed from a generated `grub.cfg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrubMenuEntry {
    /// Menu title as GRUB displays it — dequoted and unescaped.
    pub title: String,
    /// Stable identifier from `$menuentry_id_option '<id>'`, if present.
    pub id: Option<String>,
    /// `GRUB_DEFAULT`-compatible numeric index path.
    ///
    /// Top-level items get `"0"`, `"1"`, … in file order (submenus
    /// occupy an index just like plain entries). Children of a submenu
    /// get `"<parent path>><child index>"`, e.g. `"1>0"`; nesting is
    /// unbounded, e.g. `"0>1>0"`.
    pub path: String,
    /// Nesting depth: `0` = top level, `1` = inside one submenu, …
    pub depth: usize,
    /// `true` when this item is a `submenu` container (its children
    /// follow in the returned list), `false` for a bootable `menuentry`.
    pub is_submenu: bool,
}

/// Parse the menu structure out of a generated `grub.cfg`.
///
/// Returns every `menuentry` and `submenu` in file order. Content
/// outside menu blocks (`set` commands, functions, `if` blocks,
/// comments) is ignored. Lines inside a `menuentry` body are ignored.
///
/// # Errors
///
/// Returns [`BootControlError::MalformedValue`] (with `key ==
/// "grub.cfg"`) when a `menuentry`/`submenu` line has no opening `{`
/// on the same line, when a title/id quote is unterminated, or when
/// the file ends before every opened block is closed.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::grub_cfg::parse_menu_entries;
///
/// let cfg = r#"
/// menuentry 'Ubuntu' --class ubuntu $menuentry_id_option 'gnulinux-simple-abc' {
///     linux /vmlinuz root=UUID=abc ro quiet
/// }
/// "#;
/// let entries = parse_menu_entries(cfg).expect("valid cfg parses");
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].title, "Ubuntu");
/// assert_eq!(entries[0].id.as_deref(), Some("gnulinux-simple-abc"));
/// assert_eq!(entries[0].path, "0");
/// assert!(!entries[0].is_submenu);
/// ```
pub fn parse_menu_entries(cfg: &str) -> Result<Vec<GrubMenuEntry>, BootControlError> {
    let mut entries: Vec<GrubMenuEntry> = Vec::new();
    // Every open `submenu` block, innermost last — their children are
    // parsed, so scanning continues inside them.
    let mut submenus: Vec<SubmenuFrame> = Vec::new();
    // Next index at the top level (each submenu numbers its own children).
    let mut top_next_child: usize = 0;
    // Brace depth of a block whose *content* is ignored: a `menuentry`
    // body or a non-menu block like `function ... {`. While > 0, lines
    // are only scanned for unquoted braces, never for new menu items.
    let mut opaque_depth: usize = 0;

    for line in cfg.lines() {
        let trimmed = line.trim();
        // Full-line comments never affect brace tracking — a commented-out
        // `menuentry ... {` must not open a block.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if opaque_depth > 0 {
            for brace in unquoted_braces(trimmed) {
                if brace == '{' {
                    opaque_depth += 1;
                } else {
                    opaque_depth -= 1;
                    if opaque_depth == 0 {
                        break;
                    }
                }
            }
            continue;
        }

        let Some(first) = trimmed.split_whitespace().next() else {
            continue;
        };

        if first == "menuentry" || first == "submenu" {
            let parts = parse_menu_line(trimmed)?;
            let is_submenu = first == "submenu";
            let depth = submenus.len();
            let index = match submenus.last_mut() {
                Some(parent) => {
                    let i = parent.next_child;
                    parent.next_child += 1;
                    i
                }
                None => {
                    let i = top_next_child;
                    top_next_child += 1;
                    i
                }
            };
            let mut path = String::new();
            for frame in &submenus {
                path.push_str(&frame.index.to_string());
                path.push('>');
            }
            path.push_str(&index.to_string());
            entries.push(GrubMenuEntry {
                title: parts.title,
                id: parts.id,
                path,
                depth,
                is_submenu,
            });
            if parts.body_depth > 0 {
                if is_submenu {
                    submenus.push(SubmenuFrame {
                        index,
                        next_child: 0,
                    });
                } else {
                    opaque_depth = parts.body_depth;
                }
            }
        } else {
            // Non-menu line at menu level: only its unquoted braces matter.
            // `function load_video {` opens an opaque block; a bare `}`
            // closes the innermost submenu.
            for brace in unquoted_braces(trimmed) {
                if brace == '{' {
                    opaque_depth += 1;
                } else if opaque_depth > 0 {
                    opaque_depth -= 1;
                } else {
                    // Stray `}` at the very top level is tolerated.
                    submenus.pop();
                }
            }
        }
    }

    if opaque_depth > 0 || !submenus.is_empty() {
        return Err(malformed("unexpected end of file: unclosed '{' block"));
    }
    Ok(entries)
}

/// Shorthand for the parser's only error shape.
fn malformed(reason: impl Into<String>) -> BootControlError {
    BootControlError::MalformedValue {
        key: "grub.cfg".into(),
        reason: reason.into(),
    }
}

/// One open `submenu` block during the scan.
struct SubmenuFrame {
    /// The submenu's own index at its nesting level (a `path` segment).
    index: usize,
    /// Index its next direct child will receive.
    next_child: usize,
}

/// What a `menuentry`/`submenu` header line contributes to an entry.
struct MenuLineParts {
    title: String,
    id: Option<String>,
    /// Brace depth still open at the end of the header line — usually 1;
    /// 0 when the whole block opened and closed on the same line.
    body_depth: usize,
}

/// Extract title, id and brace balance from a menu header line.
fn parse_menu_line(line: &str) -> Result<MenuLineParts, BootControlError> {
    let tokens = tokenize_menu_line(line)?;
    let mut title: Option<String> = None;
    let mut id: Option<String> = None;
    let mut saw_open = false;
    let mut body_depth: usize = 0;

    let mut i = 1; // token 0 is the `menuentry`/`submenu` keyword
    while let Some(token) = tokens.get(i) {
        match token {
            Token::Word { text, quoted } => {
                // Words after the opening `{` are body content, not options.
                if !saw_open {
                    if *quoted && title.is_none() {
                        title = Some(text.clone());
                    } else if !*quoted && text == "$menuentry_id_option" {
                        if let Some(Token::Word { text: id_text, .. }) = tokens.get(i + 1) {
                            id = Some(id_text.clone());
                            i += 1;
                        }
                    }
                }
            }
            Token::Open => {
                saw_open = true;
                body_depth += 1;
            }
            Token::Close => body_depth = body_depth.saturating_sub(1),
        }
        i += 1;
    }

    if !saw_open {
        return Err(malformed(format!(
            "menu item line has no opening '{{' on the same line: {line}"
        )));
    }
    let Some(title) = title else {
        return Err(malformed(format!(
            "menu item line has no quoted title: {line}"
        )));
    };
    Ok(MenuLineParts {
        title,
        id,
        body_depth,
    })
}

/// A shell-ish token from a menu header line.
enum Token {
    /// A word; `quoted` is true when any part of it was inside quotes,
    /// which is how titles are told apart from options like `--class`.
    Word { text: String, quoted: bool },
    /// An unquoted `{`.
    Open,
    /// An unquoted `}`.
    Close,
}

/// Split a menu header line into words and unquoted braces.
///
/// Quote handling follows what `grub-mkconfig` emits: single and double
/// quotes take content verbatim, and a backslash outside quotes escapes
/// the next char — together that decodes the apostrophe idiom `'\''`
/// (close quote, escaped `'`, reopen) to a literal `'`.
fn tokenize_menu_line(line: &str) -> Result<Vec<Token>, BootControlError> {
    enum Mode {
        Unquoted,
        Single,
        Double,
    }

    fn flush(tokens: &mut Vec<Token>, word: &mut Option<(String, bool)>) {
        if let Some((text, quoted)) = word.take() {
            tokens.push(Token::Word { text, quoted });
        }
    }

    /// The word being built, started on first use. `.0` = text, `.1` = quoted.
    fn current(word: &mut Option<(String, bool)>) -> &mut (String, bool) {
        word.get_or_insert_with(|| (String::new(), false))
    }

    let mut tokens: Vec<Token> = Vec::new();
    let mut word: Option<(String, bool)> = None;
    let mut mode = Mode::Unquoted;
    let mut chars = line.chars();

    while let Some(c) = chars.next() {
        match mode {
            Mode::Unquoted => match c {
                '\'' => {
                    mode = Mode::Single;
                    current(&mut word).1 = true;
                }
                '"' => {
                    mode = Mode::Double;
                    current(&mut word).1 = true;
                }
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        current(&mut word).0.push(escaped);
                    }
                }
                '{' => {
                    flush(&mut tokens, &mut word);
                    tokens.push(Token::Open);
                }
                '}' => {
                    flush(&mut tokens, &mut word);
                    tokens.push(Token::Close);
                }
                c if c.is_whitespace() => flush(&mut tokens, &mut word),
                _ => current(&mut word).0.push(c),
            },
            Mode::Single => {
                if c == '\'' {
                    mode = Mode::Unquoted;
                } else {
                    current(&mut word).0.push(c);
                }
            }
            Mode::Double => {
                if c == '"' {
                    mode = Mode::Unquoted;
                } else {
                    current(&mut word).0.push(c);
                }
            }
        }
    }

    if !matches!(mode, Mode::Unquoted) {
        return Err(malformed(format!("unterminated quote: {line}")));
    }
    flush(&mut tokens, &mut word);
    Ok(tokens)
}

/// Yield the unquoted `{` / `}` chars of a line, in order.
///
/// Used for lines whose content is otherwise ignored; an unterminated
/// quote here is not an error — the scan just stops at end of line.
fn unquoted_braces(line: &str) -> Vec<char> {
    let mut braces = Vec::new();
    let mut chars = line.chars();
    let mut in_single = false;
    let mut in_double = false;
    while let Some(c) = chars.next() {
        if in_single {
            if c == '\'' {
                in_single = false;
            }
        } else if in_double {
            if c == '"' {
                in_double = false;
            }
        } else {
            match c {
                '\'' => in_single = true,
                '"' => in_double = true,
                '\\' => {
                    // Escaped char is literal; skip it.
                    chars.next();
                }
                '{' | '}' => braces.push(c),
                _ => {}
            }
        }
    }
    braces
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — THE CONTRACT. Do not modify tests; make them pass.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Realistic Ubuntu-style fixture: os-prober Windows entry, one
    /// submenu with two children, function + if noise around them.
    const UBUNTU_FIXTURE: &str = r#"#
# DO NOT EDIT THIS FILE
#
function load_video {
  insmod all_video
}
set default="0"
if [ x"${feature_menuentry_id}" = xy ]; then
  menuentry_id_option="--id"
fi
menuentry 'Ubuntu' --class ubuntu --class gnu-linux --class os $menuentry_id_option 'gnulinux-simple-2f1a' {
        recordfail
        load_video
        linux   /boot/vmlinuz-6.8.0 root=UUID=2f1a ro quiet splash
        initrd  /boot/initrd.img-6.8.0
}
submenu 'Advanced options for Ubuntu' $menuentry_id_option 'gnulinux-advanced-2f1a' {
        menuentry 'Ubuntu, with Linux 6.8.0' --class ubuntu $menuentry_id_option 'gnulinux-6.8.0-advanced-2f1a' {
                linux   /boot/vmlinuz-6.8.0 root=UUID=2f1a ro quiet splash
                initrd  /boot/initrd.img-6.8.0
        }
        menuentry 'Ubuntu, with Linux 6.8.0 (recovery mode)' --class ubuntu $menuentry_id_option 'gnulinux-6.8.0-recovery-2f1a' {
                linux   /boot/vmlinuz-6.8.0 root=UUID=2f1a ro recovery nomodeset
                initrd  /boot/initrd.img-6.8.0
        }
}
menuentry "Windows Boot Manager (on /dev/nvme0n1p1)" --class windows --class os $menuentry_id_option 'osprober-efi-9C2A-1111' {
        insmod part_gpt
        chainloader /EFI/Microsoft/Boot/bootmgfw.efi
}
"#;

    fn parse(cfg: &str) -> Vec<GrubMenuEntry> {
        parse_menu_entries(cfg).expect("fixture must parse cleanly")
    }

    fn malformed(cfg: &str) -> BootControlError {
        match parse_menu_entries(cfg) {
            Err(e) => e,
            Ok(v) => panic!("expected MalformedValue, got Ok({v:?})"),
        }
    }

    // ── happy path ──────────────────────────────────────────────────────────

    #[test]
    fn empty_input_returns_empty_vec() {
        assert_eq!(parse(""), Vec::new());
    }

    #[test]
    fn file_without_entries_returns_empty_vec() {
        let cfg = "set default=\"0\"\nset timeout=5\nfunction f {\n  true\n}\n";
        assert_eq!(parse(cfg), Vec::new());
    }

    #[test]
    fn single_entry_parses_title_id_path_depth() {
        let cfg = "menuentry 'Ubuntu' --class ubuntu $menuentry_id_option 'gnulinux-simple-2f1a' {\n\tlinux /vmlinuz\n}\n";
        let e = &parse(cfg)[0];
        assert_eq!(e.title, "Ubuntu");
        assert_eq!(e.id.as_deref(), Some("gnulinux-simple-2f1a"));
        assert_eq!(e.path, "0");
        assert_eq!(e.depth, 0);
        assert!(!e.is_submenu);
    }

    #[test]
    fn double_quoted_title_is_dequoted() {
        let cfg = "menuentry \"Windows Boot Manager\" --class windows {\n\tchainloader /x\n}\n";
        let e = &parse(cfg)[0];
        assert_eq!(e.title, "Windows Boot Manager");
    }

    #[test]
    fn entry_without_id_option_has_none_id() {
        let cfg = "menuentry 'Plain' {\n\ttrue\n}\n";
        assert_eq!(parse(cfg)[0].id, None);
    }

    #[test]
    fn sequential_top_level_entries_get_sequential_paths() {
        let cfg = "menuentry 'A' {\n}\nmenuentry 'B' {\n}\n";
        let v = parse(cfg);
        assert_eq!(v[0].path, "0");
        assert_eq!(v[1].path, "1");
    }

    #[test]
    fn apostrophe_escape_in_single_quoted_title() {
        // grub-mkconfig writes an apostrophe inside a single-quoted title
        // as: close quote, escaped quote, reopen quote.
        let cfg = "menuentry 'Tom'\\''s Linux' {\n\ttrue\n}\n";
        assert_eq!(parse(cfg)[0].title, "Tom's Linux");
    }

    #[test]
    fn body_lines_never_produce_entries() {
        let cfg = "menuentry 'A' {\n\techo 'menuentry fake'\n\tlinux /vmlinuz\n}\n";
        assert_eq!(parse(cfg).len(), 1);
    }

    #[test]
    fn commented_out_entries_are_ignored() {
        let cfg = "# menuentry 'Old' {\nmenuentry 'Real' {\n}\n";
        let v = parse(cfg);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].title, "Real");
    }

    #[test]
    fn crlf_input_parses() {
        let cfg = "menuentry 'A' {\r\n\tlinux /x\r\n}\r\n";
        assert_eq!(parse(cfg)[0].title, "A");
    }

    // ── submenus ────────────────────────────────────────────────────────────

    #[test]
    fn submenu_is_listed_and_children_get_arrow_paths() {
        let v = parse(UBUNTU_FIXTURE);
        let titles: Vec<&str> = v.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(
            titles,
            vec![
                "Ubuntu",
                "Advanced options for Ubuntu",
                "Ubuntu, with Linux 6.8.0",
                "Ubuntu, with Linux 6.8.0 (recovery mode)",
                "Windows Boot Manager (on /dev/nvme0n1p1)",
            ]
        );

        let adv = &v[1];
        assert!(adv.is_submenu);
        assert_eq!(adv.path, "1");
        assert_eq!(adv.depth, 0);
        assert_eq!(adv.id.as_deref(), Some("gnulinux-advanced-2f1a"));

        assert_eq!(v[2].path, "1>0");
        assert_eq!(v[2].depth, 1);
        assert!(!v[2].is_submenu);
        assert_eq!(v[3].path, "1>1");

        // Submenu occupies index 1, so Windows is index 2 — not 4.
        assert_eq!(v[4].path, "2");
        assert_eq!(v[4].depth, 0);
        assert_eq!(v[4].id.as_deref(), Some("osprober-efi-9C2A-1111"));
    }

    #[test]
    fn nested_submenus_extend_the_path() {
        let cfg = "submenu 'Outer' {\n\tsubmenu 'Inner' {\n\t\tmenuentry 'Leaf' {\n\t\t}\n\t}\n}\n";
        let v = parse(cfg);
        assert_eq!(v[0].path, "0");
        assert_eq!(v[1].path, "0>0");
        assert!(v[1].is_submenu);
        assert_eq!(v[2].path, "0>0>0");
        assert_eq!(v[2].depth, 2);
        assert_eq!(v[2].title, "Leaf");
    }

    // ── errors ──────────────────────────────────────────────────────────────

    #[test]
    fn unterminated_block_is_malformed() {
        let cfg = "menuentry 'A' {\n\tlinux /x\n";
        let e = malformed(cfg);
        assert!(
            matches!(e, BootControlError::MalformedValue { ref key, .. } if key == "grub.cfg"),
            "got: {e:?}"
        );
    }

    #[test]
    fn missing_opening_brace_is_malformed() {
        let cfg = "menuentry 'A'\n{\n}\n";
        let e = malformed(cfg);
        assert!(
            matches!(e, BootControlError::MalformedValue { .. }),
            "got: {e:?}"
        );
    }

    #[test]
    fn unterminated_title_quote_is_malformed() {
        let cfg = "menuentry 'Never closed {\n}\n";
        let e = malformed(cfg);
        assert!(
            matches!(e, BootControlError::MalformedValue { .. }),
            "got: {e:?}"
        );
    }
}
