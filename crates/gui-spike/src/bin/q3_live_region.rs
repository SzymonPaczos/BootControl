// PR 0 — Q3: Can Slint expose AT-SPI live regions?
//
// Spec wants persistent InfoBars (Setup Mode warning, daemon disconnect)
// to be announced by Orca even when focus does not move. AT-SPI live
// regions are the standard mechanism. Does Slint expose them?
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q3_live_region
// Then: open Orca, leave focus where it is, watch the InfoBar appear and
//       change. Does Orca announce? If not, that is a known framework gap
//       requiring polling-based fallback in components/info_bar.slint.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Q3 — Live region announcements";
        width: 600px;
        height: 360px;
        background: #11111b;

        in-out property <int> state: 0; // 0 = clean, 1 = warning, 2 = error

        VerticalLayout {
            padding: 24px;
            spacing: 16px;

            Text { text: "PR 0 spike — Q3"; color: #cdd6f4; font-size: 18px; }
            Text {
                text: "InfoBar materializes/changes without focus moving. Does Orca announce?";
                color: #bac2de; font-size: 12px;
            }

            // Simulated InfoBar slot — toggles via state.
            if root.state > 0 : Rectangle {
                height: 56px;
                background: root.state == 1 ? #fab387 : #f38ba8;
                accessible-role: text;
                accessible-label: root.state == 1 ? "Warning" : "Error";
                accessible-description: root.state == 1
                    ? "Secure Boot is in Setup Mode."
                    : "Daemon bootcontrold is not running on the system bus.";
                HorizontalLayout {
                    padding: 16px; spacing: 12px;
                    Text {
                        text: root.state == 1
                            ? "⚠ Secure Boot is in Setup Mode."
                            : "✕ Daemon bootcontrold is not running on the system bus.";
                        color: #11111b;
                        font-size: 14px;
                        font-weight: 700;
                        vertical-alignment: center;
                    }
                }
            }

            Rectangle { height: 16px; }

            HorizontalLayout {
                spacing: 8px;
                Button { text: "Show warning"; clicked => { root.state = 1; } }
                Button { text: "Show error";   clicked => { root.state = 2; } }
                Button { text: "Hide";         clicked => { root.state = 0; } }
            }

            Rectangle { vertical-stretch: 1; }
            Text {
                text: "Keep focus on the buttons above (do not Tab into the InfoBar).\nOrca should still announce when the InfoBar appears or its content changes.";
                color: #6c7086; font-size: 11px;
                wrap: word-wrap;
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q3 — Live region announcements");
    eprintln!("  1. Start Orca");
    eprintln!("  2. Focus a button — verify Orca reads it");
    eprintln!("  3. Click 'Show warning'");
    eprintln!("  4. Without moving focus, does Orca announce the InfoBar?");
    eprintln!("  5. Click 'Show error' — same question");
    eprintln!("  6. accerciser → Events → look for state-changed events on the InfoBar Rectangle");
    AppWindow::new()?.run()
}
