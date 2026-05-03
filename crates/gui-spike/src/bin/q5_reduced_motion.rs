// PR 0 — Q5: Does Slint propagate prefers-reduced-motion?
//
// Spec §8 requires animations to honour OS-level reduced-motion. If Slint
// does not surface this, we must read the setting from Rust (xdg-portal /
// gsettings) and feed it via a Tokens.reduced-motion property.
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q5_reduced_motion
// Then: set `gsettings set org.gnome.desktop.interface gtk-enable-animations false`
//       (or toggle "Reduce animation" in GNOME Settings → Accessibility)
//       and observe whether the animated bar still slides.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Q5 — prefers-reduced-motion";
        width: 600px;
        height: 280px;
        background: #11111b;

        in-out property <bool> moved: false;

        VerticalLayout {
            padding: 24px;
            spacing: 16px;
            Text { text: "PR 0 spike — Q5"; color: #cdd6f4; font-size: 18px; }
            Text {
                text: "Click to animate. Does Slint honour OS reduced-motion?";
                color: #bac2de; font-size: 12px;
            }
            Rectangle {
                width: 540px;
                height: 60px;
                background: #1e1e2e;
                Rectangle {
                    width: 80px;
                    height: 40px;
                    y: 10px;
                    x: root.moved ? 450px : 10px;
                    background: #cba6f7;
                    animate x { duration: 800ms; easing: ease-in-out; }
                }
            }
            Button {
                text: root.moved ? "Slide back" : "Slide right";
                clicked => { root.moved = !root.moved; }
            }
            Text {
                text: "Toggle 'Reduce animation' in GNOME Settings → Accessibility, restart app.";
                color: #6c7086; font-size: 11px;
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q5 — prefers-reduced-motion");
    eprintln!("  1. Run with default settings — verify animation plays");
    eprintln!("  2. Run: gsettings set org.gnome.desktop.interface gtk-enable-animations false");
    eprintln!("  3. Restart this binary, click the button");
    eprintln!("  4. Does the rectangle teleport (good) or slide (Slint ignored OS hint)?");
    eprintln!("  5. Also test via xdg-portal accessibility settings if available");
    AppWindow::new()?.run()
}
