// PR 0 — Q2: When `accessible-description` changes at runtime,
// does Slint emit AT-SPI property-change so Orca re-announces?
//
// The Confirmation Sheet (spec §11) restates target paths; if Orca only
// reads the description on focus, the user gets the wrong restatement.
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q2_description_change
// Then: open Orca, focus the labelled text, click "Change description",
//       observe whether Orca re-announces. Also watch accerciser for a
//       property-change::accessible-description event.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Q2 — accessible-description change emission";
        width: 560px;
        height: 360px;
        background: #11111b;

        in-out property <int> tick: 0;
        property <string> dynamic-desc: tick == 0
            ? "Restated target: /etc/default/grub"
            : (tick == 1
                ? "Restated target: /boot/loader/entries/arch.conf"
                : (tick == 2
                    ? "Restated target: /etc/kernel/cmdline"
                    : "Restated target: /var/lib/shim-signed/mok/MOK.priv"));

        VerticalLayout {
            padding: 32px;
            spacing: 20px;
            Text { text: "PR 0 spike — Q2"; color: #cdd6f4; font-size: 18px; }
            Text {
                text: "Focus the field below in Orca, then click Change.";
                color: #bac2de; font-size: 13px;
            }
            Rectangle {
                background: #1e1e2e;
                border-color: #cba6f7;
                border-width: 1px;
                height: 80px;
                accessible-role: text;
                accessible-label: "Restated target (dynamic)";
                accessible-description: root.dynamic-desc;
                VerticalLayout {
                    padding: 12px;
                    Text { text: "current description:"; color: #6c7086; font-size: 11px; }
                    Text { text: root.dynamic-desc; color: #cdd6f4; font-size: 14px; }
                }
            }
            Button {
                text: "Change description";
                clicked => { root.tick = Math.mod(root.tick + 1, 4); }
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q2 — accessible-description change emission");
    eprintln!("  1. Start Orca");
    eprintln!("  2. Tab to the description field — Orca reads first description");
    eprintln!("  3. Click 'Change description'");
    eprintln!("  4. Does Orca re-announce, or stay silent?");
    eprintln!("  5. accerciser → Events → check for 'object:property-change:accessible-description'");
    AppWindow::new()?.run()
}
