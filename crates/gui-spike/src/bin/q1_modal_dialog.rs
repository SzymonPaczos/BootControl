// PR 0 — Q1: Does Slint's PopupWindow appear modal to AT-SPI / Orca?
//
// Slint 1.8 PopupWindow is a bare container — no `background`, no
// `close-on-click-outside` properties (those arrived in 1.10+). Background
// goes on an inner Rectangle. The popup auto-closes on outside click.
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q1_modal_dialog
// Then: launch `accerciser`, navigate to this app's window, click the button,
//       examine the popup window's STATES — look for MODAL, ACTIVE, FOCUSED.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Q1 — Modal dialog AT-SPI flag";
        width: 480px;
        height: 320px;
        background: #11111b;

        sheet := PopupWindow {
            accessible-role: groupbox;
            accessible-label: "Replace Platform Key";
            accessible-description: "This is a destructive action. Restated target: /etc/secureboot/PK.auth.";
            width: 360px;
            height: 200px;
            Rectangle {
                background: #1e1e2e;
                border-color: #cba6f7;
                border-width: 2px;
                VerticalLayout {
                    padding: 16px;
                    spacing: 12px;
                    Text {
                        text: "Modal popup — verify in accerciser:";
                        color: #cdd6f4;
                        font-size: 14px;
                    }
                    Text {
                        text: "• STATES contains MODAL?\n• ROLE = groupbox or something else?\n• Orca reads label/description?";
                        color: #bac2de;
                        font-size: 12px;
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
            Text {
                text: "PR 0 spike — Q1";
                color: #cdd6f4;
                font-size: 18px;
            }
            Text {
                text: "Click to open the modal popup, then inspect via accerciser.";
                color: #bac2de;
                font-size: 13px;
            }
            Button {
                text: "Open modal";
                clicked => { sheet.show(); }
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q1 — Modal dialog AT-SPI flag");
    eprintln!("  1. Run accerciser in another terminal");
    eprintln!("  2. Click 'Open modal'");
    eprintln!("  3. Locate the popup in accerciser tree");
    eprintln!("  4. Note: ROLE, STATES (look for MODAL), accessible-name");
    eprintln!("  5. Test with Orca: does it announce as a dialog or just text?");
    AppWindow::new()?.run()
}
