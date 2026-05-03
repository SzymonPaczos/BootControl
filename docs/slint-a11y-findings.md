# Slint Accessibility Findings — BootControl GUI v2 (PR 0)

**Slint version target:** 1.14.x — Cargo.toml declares `slint = "1.8"` (caret), Cargo.lock resolves to **slint 1.14.1**, which is the version this document targets.
**Test platform target:** Linux (X11 + Wayland), Orca 46.x + AT-SPI 2 (D-Bus accessibility bus).
**Backend assumed:** Slint `winit` backend with the `accessibility` feature enabled (default), which routes accessibility through **AccessKit** (`accesskit_winit`). The Qt backend uses Qt's QAccessible bridge and is *not* the path under test.
**Status:** Research phase. This document records what published docs, source, and tracked issues say. Runtime verification on Linux with Orca + accerciser is pending and will be performed against spike binaries in `crates/gui-spike/`. The "Spec impact" lines below are conditional on those spikes confirming the documented behavior.

---

## Summary table

| Q | One-line answer | Confidence | Spike test required? | Spec impact |
|---|-----------------|------------|----------------------|-------------|
| Q1 | No. `dialog` is not a value of `accessible-role`; the AccessKit role mapping in Slint never emits `Role::Dialog`/`Role::AlertDialog`. AT-SPI `MODAL` state is therefore not surfaced from Slint markup alone. | HIGH | YES (confirm via accerciser that no `MODAL` flag is set on a `PopupWindow`). | Spec must NOT rely on Slint emitting AT-SPI `MODAL`. Use a Window-level role (closest available: `groupbox`) plus a screen-reader announcement on open. Consider OS-level second `Window` once Phase 7 brings true modal support. |
| Q2 | Unknown but likely partial. AccessKit adapter rebuilds the tree of dirty nodes when description bindings change. Whether this surfaces an AT-SPI `property-change::accessible-description` event to Orca is not documented. | LOW–MEDIUM | YES (accerciser event monitor while toggling description property). | If spike confirms re-announcement, use accessible-description for status. If not, fall back to focus-and-relabel pattern. |
| Q3 | No live-region API exists in Slint 1.14.x. `AccessibleStringProperty` enum has no Live/Polite/Assertive variants; AccessKit adapter sets no live-region attributes. | HIGH | YES (negative test — verify InfoBar text is not auto-spoken by Orca). | Spec must implement explicit "polite announce" via brief focus shift to a labeled hidden text + immediate refocus, OR delegate via `notify-send` / a libnotify-bridged toast. No CSS-aria-live equivalent. |
| Q4 | No automatic focus trap. `PopupWindow` accepts `forward-focus` for initial focus but does not block Tab from escaping. As of #7209 (closed) the popup tree is at least merged; full trap is not. | MEDIUM (close to HIGH) | YES (Tab repeatedly inside popup). | Implement manual trap: top-level `FocusScope { capture-key-pressed }` inside the popup that intercepts Tab/Shift+Tab and wraps focus to first/last focusable child. |
| Q5 | No. Slint 1.14.x exposes no global for `prefers-reduced-motion`; GitHub search for the term returns zero issues. We must read OS state from Rust and feed a global property. | HIGH | NO (negative result already conclusive). | Spec adds `Tokens.reduced_motion: bool` global. Rust side queries `org.gnome.desktop.interface enable-animations` via gsettings/zbus and `set_reduced_motion(...)` on init + on a watcher. |
| Q6 | Yes. `KeyEvent` has `text` (unicode of key) and `modifiers` (struct with `shift`, `control`, `alt`, `meta` booleans). `Key.Return` is the named constant; check `event.text == Key.Return && event.modifiers.shift`. Cross-backend behavior is not platform-doc'd but consistent in idiom. | HIGH (docs) / MEDIUM (X11 vs Wayland parity) | YES (one tap each: Return, Shift+Return, on X11 and Wayland). | Spec uses `event.text == Key.Return && !event.modifiers.shift` for "submit" and `&& event.modifiers.shift` for the alternate action. |
| Q7 | No runtime swap. Globals are compile-time singletons; only individual `out`/`in-out` properties may be mutated at runtime via generated `set_*` setters, which propagate reactively. | HIGH | NO (already conclusive from Rust API + docs). | Spec must structure `Tokens` as a single global with all theme properties as in-out, and switch contrast by setting every property at once from Rust (one transaction). No "swap whole global" path. |

---

## Q1 — Modal dialog AT-SPI flag

**Question.** Does `accessible-role: dialog` on a Slint `Window`/`PopupWindow` translate to the AT-SPI `MODAL` state visible to Orca? If yes, version requirement.

**Documentation says.**
The full set of `accessible-role` values is enumerated at <https://docs.slint.dev/latest/docs/slint/reference/global-structs-enums/> and at the rustdoc <https://docs.rs/i-slint-core/1.14.1/i_slint_core/items/enum.AccessibleRole.html>:

> "none, button, checkbox, combobox, groupbox, image, list, slider, spinbox, tab, tab-list, tab-panel, text, table, tree, progress-indicator, text-input, switch, list-item, radio-button"

There is **no `dialog`, `alertdialog`, `alert`, `status`, or `live-region` variant**. The rustdoc confirms 19 numbered variants ending at `ListItem = 18`; the enum is `#[non_exhaustive]` so future versions could add more, but 1.14.1 does not.

The Window reference at <https://docs.slint.dev/latest/docs/slint/reference/window/window/> lists 12 properties — none of which is `modal`, `accessible-role`, or any focus-management property.

The PopupWindow reference at <https://docs.slint.dev/latest/docs/slint/reference/window/popupwindow/> exposes only `close-policy` plus `show()`/`close()`. No modality flag.

**Source code says.** Per <https://github.com/slint-ui/slint/blob/master/internal/backends/winit/accesskit.rs>, the `match item.accessible_role()` function maps Slint roles into `accesskit::Role::Button`, `Role::TextInput`, `Role::CheckBox`, etc. The fetched summary states verbatim that the mapping function does **not map any role to `Dialog` or `AlertDialog`**, and "no modal or dialog-specific flags are set on nodes." AccessKit's Linux adapter is what would translate a `Role::Dialog` to AT-SPI's `ROLE_DIALOG` plus `STATE_MODAL`; since the upstream side never emits that role, AT-SPI never sees `MODAL`.

**GitHub issues / PRs.**
- [#6607 — "Modal windows / dialogs"](https://github.com/slint-ui/slint/issues/6607) — **OPEN** feature request from Oct 2024 by @Enyium. Body proposes a `new_modal()` constructor that on Windows would call `winit::WindowAttributesExtWindows::with_owner_window` and disable the parent. Notes "On non-Windows platforms, the functionality would have no effect due to winit limitations." No mention of AT-SPI `MODAL`, no Linux path, no maintainer comment, no linked PR, no milestone.
- [Discussion #6028 — "Modal Dialogs?"](https://github.com/slint-ui/slint/discussions/6028) — Maintainer position (paraphrased in fetch): PopupWindow is "the equivalent of a HTML popup implemented with a span tag" — not modal, no focus trap, no OS-level modal flag.
- [#7209 — "Popups should expose their accessibility tree"](https://github.com/slint-ui/slint/issues/7209) — **CLOSED (completed)**. Fixed merging of the popup subtree into the parent's accessibility tree. Does not add a dialog role or modal state.

**Confidence-from-research.** **HIGH.** Three independent sources (enum docs, rustdoc, AccessKit adapter source) agree.

**Implication for spec.**
- Do not write `accessible-role: dialog` — it is not a valid value and will be rejected by the Slint compiler.
- Closest workable role on a popup container is `groupbox` (or leave default `none` and rely on `accessible-label`).
- Do not assume Orca will say "dialog" or treat the popup as modal automatically. The v2 spec must:
  1. Announce dialog open via an explicit cue (focus a labeled element whose `accessible-label` reads "<title> dialog, <N> controls" so that Orca speaks the construct).
  2. Implement the focus trap manually (see Q4).
  3. Track #6607 for true OS-modal once it lands and revisit on Linux (it's currently a Windows-only proposal anyway).

---

## Q2 — `accessible-description` change emission

**Question.** When a Slint property bound to `accessible-description` changes at runtime, does Slint emit an AT-SPI `property-change::accessible-description` event so a screen reader re-announces? Or does the description only get read once on focus?

**Documentation says.** The property reference at <https://docs.slint.dev/latest/docs/slint/reference/common/#accessible-description> states only the type (`string`) and that it provides "the description for the current element." No mention of change events, re-announcement, or AT-SPI signal mapping.

**Source code says.** The AccessKit adapter (`internal/backends/winit/accesskit.rs`, fetched summary) uses a push-based `TreeUpdate` notification model. Two paths:
- `rebuild_tree()` — full rebuild on structural change.
- `rebuild_tree_of_dirty_nodes()` — partial update driven by `PropertyTracker`, used when bound property values change.

When the description changes, the dirty-node path runs and calls `node.set_description(...)`. AccessKit then publishes a `TreeUpdate` to its platform adapter. On Linux, AccessKit's Unix adapter (zbus-based AT-SPI implementation) is responsible for translating the diff to `property-change::accessible-description`. **Whether AccessKit's Unix adapter actually emits that specific AT-SPI signal in 2026 is not documented in the Slint repo and we did not verify in the AccessKit Linux adapter source.** This is the gap.

**GitHub issues / PRs.**
- [#2895 — "Accessibility: Support text input widgets"](https://github.com/slint-ui/slint/issues/2895) — **OPEN**, no comments visible, no mention of description change events.
- [#8732 — "Accessibility issues with text fields on Windows"](https://github.com/slint-ui/slint/issues/8732) — **CLOSED (duplicate of #2895)**, Windows-specific, reporter notes screen readers re-announce the *entire field value on each keystroke*, suggesting the value-change path *does* fire (even too aggressively). Tangential evidence that description-change might also fire.
- No issue specifically tracking `accessible-description` re-announcement on Linux.

**Confidence-from-research.** **LOW–MEDIUM.** Slint dirties the node and pushes a `TreeUpdate`, so the upstream half is known to fire. Whether Orca re-announces on the AT-SPI side is unverified.

**Implication for spec.**
- **If spike confirms re-announcement on Linux:** Use `accessible-description` for transient status messages on the focused control (e.g. on the Save button: "Saved" / "Reverted").
- **If spike shows no re-announcement:** Fall back to a hidden labeled element that we briefly focus. That is the same workaround Q3 needs.
- Do not assume parity with Windows — if anything, AT-SPI is *more* sensitive to property-change signals than UIA, but only if AccessKit emits them.

---

## Q3 — Live regions

**Question.** Can Slint expose AT-SPI live regions (e.g. `aria-live=polite` equivalent) such that an InfoBar appearing mid-flow gets read out without focus moving? Is there any Slint-provided live-region API or workaround pattern?

**Documentation says.** Search of the official docs (common reference, global enums, best practices) returns zero hits for "live region", "aria-live", "polite", or "assertive". The accessibility section of the best-practices guide (<https://docs.slint.dev/latest/docs/slint/guide/development/best-practices/>) verbatim:

> "When designing custom components, consider early on to declare accessibility properties. At least a role, possibly a label, as well as actions."

No live-region pattern is mentioned.

**Source code says.** The `AccessibleStringProperty` enum (per the fetched core `accessibility.rs`) has 20 string properties — Checkable, Checked, DelegateFocus, Description, Enabled, Expandable, Expanded, Id, ItemCount, ItemIndex, ItemSelectable, ItemSelected, Label, PlaceholderText, ReadOnly, Value, ValueMaximum, ValueMinimum, ValueStep — and **no live-region variant** (no Live, Politeness, Atomic, Relevant). The AccessKit adapter summary explicitly notes "No live region attributes are exposed through AccessKit." AccessKit itself (upstream) does have a `live: Live` field on nodes (with `Off`/`Polite`/`Assertive`), but Slint's adapter never sets it.

**GitHub issues / PRs.** Web searches for `slint live region`, `slint aria-live`, `slint announcement` return no results in the slint-ui/slint repo. Closest is #2895 (text-input a11y, open) and #7546 (ListView a11y delay, closed) — neither is a live-region request.

**Confidence-from-research.** **HIGH** that the feature is absent. The enum has no slot for it; the adapter sets nothing for it; no issue tracks it.

**Implication for spec.**
- Do not rely on Slint markup alone for InfoBar announcement.
- Workaround pattern: when an InfoBar shows, programmatically save current focus, briefly move focus to the InfoBar's labeled root (which forces Orca to read its `accessible-label`), then restore focus on a 100–200 ms timer. This is a hack and may steal selection. Test thoroughly.
- Alternative: emit a desktop notification via `org.freedesktop.Notifications` (zbus). Orca normally announces notifications. This bypasses Slint accessibility entirely. Cleaner for non-blocking status, but does not work for in-window confirmations.
- File a Slint feature request post-PR-0 to add `accessible-live: polite|assertive` mapped to AccessKit `Live`.

---

## Q4 — Modal focus trap

**Question.** Does Slint's `PopupWindow` (with `close-on-click-outside: false` and modality) trap Tab focus inside it, so Tab cycles within the popup and never escapes to the parent?

**Documentation says.** PopupWindow doc (<https://docs.slint.dev/latest/docs/slint/reference/window/popupwindow/>) lists only `close-policy` and `show()`/`close()`. No modality, no focus-trap property. The Focus Handling guide (<https://docs.slint.dev/latest/docs/slint/guide/development/focus/>) mentions `forward-focus` for initial focus only.

**Source code says.** Not directly inspected, but the behavior is governed by `internal/core/items/popup_window.rs` and the Tab navigation logic in `internal/core/window.rs` / FocusScope handling. Discussion #6028 (modal dialogs) characterizes PopupWindow as the equivalent of an HTML span overlay — visual only, no input modality.

**GitHub issues / PRs.**
- [#2911 — "PopupWindow focus handling"](https://github.com/slint-ui/slint/issues/2911) — **CLOSED via PR #7014**. Resolved three issues: (1) parent doesn't show focus ring while popup is up; (2) initial focus transfers to popup; (3) Tab works between buttons inside the popup. Fix uses `forward-focus`. No mention that Tab is *prevented* from escaping, only that initial focus and intra-popup Tab work.
- [Discussion #6028 — "Modal Dialogs?"](https://github.com/slint-ui/slint/discussions/6028) — Maintainer states no modal blocking; user implemented Win32-specific hack.
- [#6607 — "Modal windows / dialogs"](https://github.com/slint-ui/slint/issues/6607) — Open feature request, no Linux path proposed.
- GitHub search `is:issue focus trap` against slint-ui/slint returns zero hits.

**Confidence-from-research.** **MEDIUM** approaching HIGH. No documentation, source comment, or PR claims an automatic Tab trap. The closed #2911 fixed initial focus and intra-popup Tab, not escape prevention.

**Implication for spec.**
Implement the trap manually:

```slint
PopupWindow {
    close-policy: no-auto-close;
    FocusScope {
        capture-key-pressed(event) => {
            if (event.text == Key.Tab) {
                if (event.modifiers.shift && first-control.has-focus) {
                    last-control.focus();
                    return accept;
                } else if (!event.modifiers.shift && last-control.has-focus) {
                    first-control.focus();
                    return accept;
                }
            }
            if (event.text == Key.Escape) { dialog-cancel(); return accept; }
            return reject;
        }
        // ... popup content; first-control + last-control are root and tail focusables
    }
}
```

`capture-key-pressed` was confirmed available on FocusScope (per discussion #4231 update Nov 2025). It fires before child widgets see the event, so the trap intercepts Tab even when a LineEdit inside has focus.

---

## Q5 — `prefers-reduced-motion`

**Question.** Does Slint propagate the OS-level reduced-motion setting (`gsettings org.gnome.desktop.interface enable-animations`, XDG portal `prefers-reduced-motion`) to a queryable property/global? Or do we feed it from Rust?

**Documentation says.** Searching the entire Slint docs for "prefers-reduced-motion", "reduced motion", "enable-animations", "gtk-enable-animations" returns zero results. The Animations reference (<https://releases.slint.dev/1.7.1/docs/slint/src/language/syntax/animations>) describes `animate { duration: ...; }` blocks but offers no global animation toggle and no system-preference hook.

**Source code says.** No reference in the AccessKit adapter, the winit backend, or core to GTK settings, `enable-animations`, or `prefers-reduced-motion`. Slint reads no XDG portal settings (the codebase does not depend on `ashpd` or similar). Animations run unconditionally if defined.

**GitHub issues / PRs.** GitHub search `prefers-reduced-motion` against slint-ui/slint returns zero hits. No accessibility-animation issue exists.

**Confidence-from-research.** **HIGH.** This feature is absent. Negative result is conclusive (no docs, no code path, no issue, no discussion).

**Implication for spec.**
Implement on the Rust side:

1. Add a global to Slint markup:
   ```slint
   export global Tokens {
       in property <bool> reduced-motion: false;
       in property <duration> default-anim: reduced-motion ? 0ms : 150ms;
       // ...
   }
   ```
2. On startup in Rust, query `org.gnome.desktop.interface enable-animations` via either:
   - `gio::Settings::new("org.gnome.desktop.interface").boolean("enable-animations")`, or
   - shelling to `gsettings get` (less ideal), or
   - zbus to `org.freedesktop.portal.Settings.Read("org.gnome.desktop.interface", "enable-animations")` (preferred, sandbox-safe).
3. Call `app.global::<Tokens>().set_reduced_motion(!enable_animations)`.
4. Subscribe to changes via `Settings::connect_changed` or the portal's `SettingChanged` D-Bus signal and update reactively.
5. Every animation in `.slint` reads `Tokens.default-anim` (or a category-specific token) so the toggle disables all motion at once.

No Slint version requirement; this is purely a host-side concern.

---

## Q6 — `Shift+Return` distinguishability

**Question.** In `KeyEvent`, is `Shift+Return` distinguishable from plain `Return` reliably across Wayland and X11?

**Documentation says.** The Key Handling overview (<https://docs.slint.dev/latest/docs/slint/reference/keyboard-input/overview/>) lists three KeyEvent fields:

> - `text` (string): The unicode representation of the key pressed
> - `modifiers` (KeyboardModifiers): The keyboard modifiers active during the key press
> - `repeat` (bool): True for repeated key press events (key held down); always false for releases

Named keys include `Return` in the `Key` namespace. The `KeyboardModifiers` struct exposes four boolean modifiers in the language: **Meta, Control, Shift, Alt**. In Rust the corresponding struct is `slint::platform::WindowEvent`-adjacent `KeyboardModifiers` with `shift`, `control`, `alt`, `meta` booleans.

Idiomatic check (from the Slint docs and discussion examples):

```slint
key-pressed(event) => {
    if (event.text == Key.Return && !event.modifiers.shift) {
        submit();
        return accept;
    } else if (event.text == Key.Return && event.modifiers.shift) {
        new-line();
        return accept;
    }
    return reject;
}
```

**Source code says.** Slint's KeyEvent originates in `i_slint_core/input.rs` (per docs.rs link from the search). The winit backend translates `winit::event::KeyEvent` (which carries logical key + physical key + modifier state from the OS) into Slint's text/modifiers form. winit's modifier handling is consistent across X11 and Wayland in winit 0.30+. Slint 1.14.1 uses winit 0.30+.

**GitHub issues / PRs.**
- [Discussion #3189 — "Adding key-code and scan-code fields to KeyEvent struct"](https://github.com/slint-ui/slint/discussions/3189) — Maintainer @hunger: *"The next winit version will have a greatly revamped key event system. We should be able to benefit from the extra information they provide and should be able to improve our key events accordingly."* Maintainer @ogoffart notes Ctrl/Cmd combinations already produce layout-independent results. This is about *layout-independence* (e.g. AZERTY vs QWERTY), not about Shift+Return discriminability — those are orthogonal.
- [Discussion #4231 — "Intercepting key events regardless of focus"](https://github.com/slint-ui/slint/discussions/4231) — Confirmed availability of `FocusScope.capture-key-pressed` as of Nov 2025.
- No open issue reports Shift+Return being indistinguishable from Return on any backend.

**Confidence-from-research.** **HIGH** for the API shape (text + modifiers are separate fields, idiom is documented). **MEDIUM** for X11/Wayland *consistency* — winit upstream is consistent, but we have not verified empirically.

**Implication for spec.**
- Use `event.text == Key.Return` for the keypress identity check (do not depend on physical key codes).
- Use `event.modifiers.shift` for the modifier branch.
- Spike test must include both X11 and Wayland sessions to confirm the modifier flag is set on both. If Wayland reports an empty `text` for some logical-Return keypresses (a known winit quirk on some compositors with alternate layouts), we need a fallback comparing against `Key.Enter` as well.

---

## Q7 — Runtime `global` override

**Question.** Slint has the `global` keyword for shared properties. Can we *swap* a global at runtime — e.g. re-bind every `Tokens.surface` reference to a high-contrast value mid-session — or is `global` static-bound at compile time and the only runtime mechanism is to mutate individual `out` properties of the global?

**Documentation says.** The Globals guide (<https://docs.slint.dev/latest/docs/slint/guide/language/coding/globals/>) states verbatim:

> "Declare a global singleton with `global Name { /* .. properties or callbacks .. */ }` to make properties and callbacks available throughout the entire project. Access them using `Name.property`."

Plus the constraint: *"Global singletons are not shared between windows. This means you may need to initialize the global callback and properties for each window instance you create in your application."*

The Rust trait `Global` (<https://docs.rs/slint/latest/slint/trait.Global.html>) exposes only `fn get(component: &Component) -> Self`. No `replace`, no `swap`, no factory. Setters like `set_background_color` are compiler-generated per-property accessors — they mutate the existing property, they do not rebind the global.

**Source code says.** Slint's compiler treats globals as compile-time singletons whose property table is allocated per component instance. The reactive property system (per the DeepWiki "Property System & Reactive Bindings" page) automatically propagates changes when an `in`/`in-out` property is set; bindings that reference `Tokens.surface` re-evaluate whenever `Tokens.surface` changes. There is no runtime indirection that would let you replace `Tokens` with a different struct.

**GitHub issues / PRs.** PR #1093 ("Rework the global singleton section in the language reference") confirmed the conceptual model but did not add any "swap" API. No issue tracks runtime global replacement.

**Confidence-from-research.** **HIGH.** The Rust API has no swap entry point and the language model has no indirection layer.

**Implication for spec.**
- Define a single `Tokens` global with every theme value as `in-out property`.
- High-contrast switch is a Rust-side function that calls every `Tokens::set_*` setter in sequence (one batch). Reactive bindings flush downstream.
- Do not design components against multiple competing `Tokens` globals expected to be swapped — the model is "one Tokens, mutated".
- For semantic theme variants ("default", "high-contrast"), keep the data in Rust and write a single `apply_theme(handle, &Theme)` helper.

---

## Spike test protocol

For each question, this is what `crates/gui-spike/` must verify on Linux. Run inside a GNOME/KDE session with Orca enabled (`Super+Alt+S`) and `accerciser` open in a second window.

**Q1 — Modal dialog AT-SPI flag.**
- Build a spike binary that opens a `PopupWindow` containing two buttons.
- In accerciser, navigate to the popup node. Look at "Interfaces" → "Accessible" → States.
- **Pass:** `MODAL` listed in States. **Fail (expected):** no `MODAL` state. If fail, document and proceed with manual focus trap (Q4) + announcement workaround.

**Q2 — accessible-description re-announce.**
- Spike binary with a Button that has `accessible-description` bound to a property. Bind a timer that toggles the description every 2 s while the button is focused.
- In accerciser → Event Monitor, subscribe to `object:property-change`. In Orca, listen for re-speech.
- **Pass:** every property toggle triggers `property-change:accessible-description` event AND Orca re-speaks. **Partial:** event fires, Orca silent (Orca filtering — fine, change strategy). **Fail:** no event at all (we must use focus-jiggle workaround).

**Q3 — Live regions.**
- Spike binary with an InfoBar Text whose content changes every 5 s while focus stays in a TextInput elsewhere.
- Listen with Orca: does it speak the InfoBar update? Inspect in accerciser whether the InfoBar node has `live` attribute.
- **Pass (unexpected):** Orca speaks; node has live attribute. **Fail (expected):** silence; no live attribute. Then: prototype focus-jiggle and `org.freedesktop.Notifications` workarounds and pick one.

**Q4 — Modal focus trap.**
- Spike: PopupWindow with three buttons + one LineEdit, plus the `capture-key-pressed` FocusScope wrapper from §Q4.
- Test: open popup, press Tab 10× — focus must cycle within four controls and never reach parent. Press Shift+Tab 10× — same in reverse. Press Escape — popup closes, focus returns to opener.
- **Pass:** all three behaviors. **Fail:** Tab escapes — refine the FocusScope position and wrap detection logic.

**Q5 — prefers-reduced-motion.**
- Set `gsettings set org.gnome.desktop.interface enable-animations false`.
- Run spike binary that animates a rectangle's position via `animate x { duration: 500ms; }` controlled by `Tokens.default-anim`.
- Spike's Rust startup must read the gsetting (or portal `Settings.Read`) and call `set_reduced_motion(true)`.
- **Pass:** rectangle snaps without animation. Toggle gsetting back to true at runtime; without restarting, the spike must observe the change (portal SettingChanged) and animations resume.

**Q6 — Shift+Return distinguishability.**
- Spike: a `FocusScope` that prints to stderr on `key-pressed`: `text="<text>" shift=<bool> ctrl=<bool>`.
- Run once under X11 (`GDK_BACKEND=x11`, `WINIT_UNIX_BACKEND=x11`) and once under Wayland (`WAYLAND_DISPLAY` set).
- Press Return alone, then Shift+Return, on each.
- **Pass:** in all four cases the shift bit reflects reality and `text` is `\n` (or `Key.Return`'s sentinel) on both. **Fail:** if Wayland reports empty text on Shift+Return — fall back to checking the named-key path.

**Q7 — Runtime global override.**
- Spike: `Tokens.surface` is a brush bound across 10 rectangles. A button calls a Rust handler that calls `set_surface(<new color>)`.
- **Pass:** all 10 rectangles update synchronously on click. (Already considered conclusive from API; spike is just sanity.)

---

## Spike binaries (in `crates/gui-spike/`)

All 7 spike binaries exist and compile clean on macOS (no Linux compile-time deps). Each is a self-contained `slint::slint!`-macro app that exercises the specific framework feature. Run on Linux with `accerciser` + Orca to record runtime results.

| Question | Binary | Run command |
|---|---|---|
| Q1 — Modal AT-SPI flag | `q1_modal_dialog` | `cargo run -p bootcontrol-gui-spike --bin q1_modal_dialog` |
| Q2 — `accessible-description` change | `q2_description_change` | `cargo run -p bootcontrol-gui-spike --bin q2_description_change` |
| Q3 — Live regions | `q3_live_region` | `cargo run -p bootcontrol-gui-spike --bin q3_live_region` |
| Q4 — Focus trap | `q4_focus_trap` | `cargo run -p bootcontrol-gui-spike --bin q4_focus_trap` |
| Q5 — `prefers-reduced-motion` | `q5_reduced_motion` | `cargo run -p bootcontrol-gui-spike --bin q5_reduced_motion` |
| Q6 — `Shift+Return` | `q6_shift_return` | `cargo run -p bootcontrol-gui-spike --bin q6_shift_return` |
| Q7 — Runtime global override | `q7_global_override` | `cargo run -p bootcontrol-gui-spike --bin q7_global_override` |

Each binary's `main()` prints to stderr a numbered checklist of what the operator must observe in `accerciser` / Orca / both X11 and Wayland sessions. After running, append a `## Runtime verification` subsection to the matching `Q*` section above with PASS/FAIL/PARTIAL + notes. Tests that the research already marked HIGH-confidence + no-spike-needed (Q5, Q7) are still useful as sanity binaries.

See [`crates/gui-spike/README.md`](../crates/gui-spike/README.md) for the full operator setup (installing `orca` + `accerciser`, AT-SPI bus check, X11/Wayland switch).

---

## Open questions — resolved

1. **Slint version pin.** **Resolved: keep `slint = "1.8"` caret in [`crates/gui/Cargo.toml`](../crates/gui/Cargo.toml).** Caret allows ≥ 1.8 < 2.0 so the lockfile-resolved 1.14.1 stays valid; there is no upside to tightening to `"1.14"` since we don't depend on 1.14-specific syntax in production code. Spike crate matches (`slint = "1.8"`). Rationale: minimal diff, lockfile is the source of truth.

2. **AccessKit Linux completeness.** **Resolved: option (b) — focus-jiggle workaround in v2; file upstream issue in parallel.** Aligns with `GUI_V2_SPEC_v2.md` shipping cadence. Track upstream in a new `docs/slint-upstream-tracking.md` (file in PR 6 or later — not PR 0 scope).

3. **Modal strategy.** **Resolved: accept modal-by-convention.** Spec already does — `PopupWindow` + manual focus trap (added to PR 4 implementation note) + Rust-side AT-SPI announcement on open. Linux OS-level modality is not on Slint's roadmap and we should not wait.

4. **Reduced-motion source.** **Resolved: GNOME first, KDE follow-up.** PR 1 reads `org.gnome.desktop.interface enable-animations` via `gsettings`/`zbus`. KDE support deferred to a post-v2 issue (acceptable: KDE users get default animations, comparable to status quo). The token `Tokens.reduced_motion` is set once at startup; runtime toggling deferred (XDG portal `SettingChanged` watcher is also a follow-up).

5. **Spike crate output.** **Resolved: DONE.** All 7 binaries written and compile-tested in this PR. `crates/gui-spike/` is the deliverable.

---

## File written
- `/Users/szymonpaczos/DevProjects/BootControl/docs/slint-a11y-findings.md`
