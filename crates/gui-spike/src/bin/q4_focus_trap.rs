// PR 0 — Q4: Does Slint's PopupWindow trap Tab focus?
//
// Spec §11 requires modal Confirmation Sheets to keep Tab cycling inside
// the popup. If Slint does not trap focus, Tab can land on the parent
// window, breaking the modal mental model.
//
// Note: Slint 1.8 PopupWindow auto-closes on outside click; that does NOT
// affect this test — we are testing keyboard-only behaviour.
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q4_focus_trap
// Then: keyboard-only — open the popup, hit Tab repeatedly, see whether
//       focus ever leaves the popup back to the main window's button.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Q4 — Modal focus trap";
        width: 520px;
        height: 360px;
        background: #11111b;

        sheet := PopupWindow {
            accessible-role: groupbox;
            width: 400px;
            height: 240px;
            Rectangle {
                background: #1e1e2e;
                border-color: #cba6f7;
                border-width: 2px;
                VerticalLayout {
                    padding: 16px;
                    spacing: 12px;
                    Text {
                        text: "Tab through these buttons. Does focus stay inside?";
                        color: #cdd6f4;
                        font-size: 13px;
                    }
                    HorizontalLayout {
                        spacing: 8px;
                        Button { text: "Cancel"; }
                        Button { text: "Apply"; }
                        Button { text: "More"; }
                    }
                    Text {
                        text: "(click outside to close)";
                        color: #6c7086;
                        font-size: 11px;
                    }
                }
            }
        }

        VerticalLayout {
            padding: 32px;
            spacing: 16px;
            Text { text: "PR 0 spike — Q4"; color: #cdd6f4; font-size: 18px; }
            Text {
                text: "Open the popup, then Tab. Should never reach the buttons here.";
                color: #bac2de; font-size: 13px;
            }
            HorizontalLayout {
                spacing: 8px;
                Button { text: "Open popup"; clicked => { sheet.show(); } }
                Button { text: "Outside Button A"; }
                Button { text: "Outside Button B"; }
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q4 — Modal focus trap");
    eprintln!("  1. Click 'Open popup' (or press Enter on the focused button)");
    eprintln!("  2. Hit Tab 8-10 times");
    eprintln!("  3. Does focus stay inside the popup, or escape to 'Outside Button A/B'?");
    eprintln!("  4. If focus escapes, we need manual focus-trap (see PR 4 implementation note)");
    AppWindow::new()?.run()
}
