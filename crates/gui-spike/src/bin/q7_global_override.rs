// PR 0 — Q7: Can Slint `global` properties be overridden at runtime?
//
// Spec §8 requires high-contrast / Mocha theme swap to be a runtime
// operation (no recompile). If `global` properties are mutable from Rust
// and components react, we can ship a single Tokens global. If not, we
// need to thread theme state through every component manually.
//
// Run:  cargo run -p bootcontrol-gui-spike --bin q7_global_override
// Then: click "Toggle high contrast" — surface, accent, text colours
//       should swap. If they do, runtime override works.

slint::slint! {
    import { Button } from "std-widgets.slint";

    export global Tokens {
        in-out property <color> surface: #1e1e2e;
        in-out property <color> on-surface: #cdd6f4;
        in-out property <color> accent: #cba6f7;
        in-out property <color> on-accent: #11111b;
    }

    component DemoCard inherits Rectangle {
        in property <string> label;
        background: Tokens.surface;
        border-color: Tokens.accent;
        border-width: 1px;
        height: 60px;
        HorizontalLayout {
            padding: 12px;
            Text {
                text: label;
                color: Tokens.on-surface;
                font-size: 14px;
                vertical-alignment: center;
            }
        }
    }

    export component AppWindow inherits Window {
        title: "Q7 — Runtime global override";
        width: 600px;
        height: 360px;
        background: Tokens.surface;

        callback toggle-contrast();

        VerticalLayout {
            padding: 24px;
            spacing: 12px;
            Text {
                text: "PR 0 spike — Q7";
                color: Tokens.on-surface;
                font-size: 18px;
            }
            Text {
                text: "Toggle high contrast. Do all colours swap live?";
                color: Tokens.on-surface;
                font-size: 12px;
            }
            DemoCard { label: "Card 1 — surface + on-surface"; }
            DemoCard { label: "Card 2 — same tokens"; }
            Rectangle {
                background: Tokens.accent;
                height: 40px;
                Text {
                    text: "Accent fill, on-accent text";
                    color: Tokens.on-accent;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                    font-size: 13px;
                }
            }
            Button {
                text: "Toggle high contrast";
                clicked => { root.toggle-contrast(); }
            }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    eprintln!("Q7 — Runtime global override");
    eprintln!("  1. Note the initial Catppuccin Mocha colours");
    eprintln!("  2. Click 'Toggle high contrast'");
    eprintln!("  3. All four token references should re-render");
    eprintln!("  4. If only some update, global override is partial — bug?");

    let app = AppWindow::new()?;
    let app_weak = app.as_weak();
    let mut high = false;
    app.on_toggle_contrast(move || {
        if let Some(app) = app_weak.upgrade() {
            let tokens = app.global::<Tokens>();
            high = !high;
            if high {
                tokens.set_surface(slint::Color::from_rgb_u8(0x00, 0x00, 0x00));
                tokens.set_on_surface(slint::Color::from_rgb_u8(0xff, 0xff, 0xff));
                tokens.set_accent(slint::Color::from_rgb_u8(0xff, 0xd8, 0x6b));
                tokens.set_on_accent(slint::Color::from_rgb_u8(0x00, 0x00, 0x00));
                eprintln!("[TOKENS] switched to high-contrast palette");
            } else {
                tokens.set_surface(slint::Color::from_rgb_u8(0x1e, 0x1e, 0x2e));
                tokens.set_on_surface(slint::Color::from_rgb_u8(0xcd, 0xd6, 0xf4));
                tokens.set_accent(slint::Color::from_rgb_u8(0xcb, 0xa6, 0xf7));
                tokens.set_on_accent(slint::Color::from_rgb_u8(0x11, 0x11, 0x1b));
                eprintln!("[TOKENS] switched to Catppuccin Mocha");
            }
        }
    });
    app.run()
}
