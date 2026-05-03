// PR 0 — Q6: Does Slint distinguish Shift+Return from plain Return?
//
// Spec §15 binds Ctrl+Shift+Return = Apply, Ctrl+Return = activate
// destructive (after type-to-confirm). If the modifier mask is unreliable
// across X11/Wayland, we cannot ship this keymap.
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q6_shift_return
// Then: focus the FocusScope, press Return, Shift+Return, Ctrl+Return,
//       Ctrl+Shift+Return. Each press prints to stderr — verify modifier
//       fields match expectation on both X11 and Wayland.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Q6 — Shift+Return distinguishability";
        width: 600px;
        height: 320px;
        background: #11111b;

        callback key-event(string, bool, bool, bool, bool); // text, shift, ctrl, alt, meta
        in-out property <string> last-event: "(focus the area below and press a key — full event in stderr)";

        VerticalLayout {
            padding: 24px;
            spacing: 16px;
            Text { text: "PR 0 spike — Q6"; color: #cdd6f4; font-size: 18px; }
            Text {
                text: "Click into the box, then test: Return, Shift+Return, Ctrl+Return, Ctrl+Shift+Return. Each event prints to stderr.";
                color: #bac2de; font-size: 12px;
                wrap: word-wrap;
            }
            scope := FocusScope {
                width: 540px;
                height: 80px;
                key-pressed(event) => {
                    root.last-event = "text=\"" + event.text + "\" shift=" +
                        (event.modifiers.shift ? "T" : "F") + " ctrl=" +
                        (event.modifiers.control ? "T" : "F") + " alt=" +
                        (event.modifiers.alt ? "T" : "F") + " meta=" +
                        (event.modifiers.meta ? "T" : "F");
                    root.key-event(event.text, event.modifiers.shift, event.modifiers.control, event.modifiers.alt, event.modifiers.meta);
                    accept
                }
                Rectangle {
                    background: scope.has-focus ? #1e1e2e : #181825;
                    border-color: scope.has-focus ? #cba6f7 : #313244;
                    border-width: 2px;
                    Text {
                        text: scope.has-focus ? "FOCUSED — press a key" : "Click to focus";
                        color: #cdd6f4;
                        font-size: 13px;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }
            }
            Rectangle {
                background: #11111b;
                border-color: #45475a;
                border-width: 1px;
                height: 60px;
                VerticalLayout {
                    padding: 10px;
                    Text { text: "Last event:"; color: #6c7086; font-size: 11px; }
                    Text { text: root.last-event; color: #a6e3a1; font-size: 12px; }
                }
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q6 — Shift+Return distinguishability");
    eprintln!("  1. Click into the focus box");
    eprintln!("  2. Press: Return, Shift+Return, Ctrl+Return, Ctrl+Shift+Return");
    eprintln!("  3. Each press prints below — also captured below to stderr");
    eprintln!("  4. Verify: modifier fields are correct AND consistent on X11 + Wayland");
    eprintln!();

    let app = AppWindow::new()?;
    app.on_key_event(|text: slint::SharedString, shift, ctrl, alt, meta| {
        eprintln!(
            "[KEY] text={:?} shift={} ctrl={} alt={} meta={}",
            text.as_str(), shift, ctrl, alt, meta
        );
    });
    app.run()
}
