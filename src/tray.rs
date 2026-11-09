use crate::ui::State;
use rune::{Align, AppHandle, DisplayTarget, OverlayPlacement, ScreenEdge};
use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

pub fn install(window: AppHandle<State>, panel: AppHandle<State>) {
    let fallback = window.clone();
    window.update(move |s, _| {
        match TrayIconBuilder::new()
            .with_title("Reoclo")
            .with_tooltip("Reoclo local tunnels")
            .build()
        {
            Ok(tray) => s.tray = Some(tray),
            Err(error) => {
                s.tray_error = Some(format!(
                    "Menu bar unavailable: {error}. Use this window to manage tunnels."
                ));
                fallback.set_dock_icon(true);
                fallback.open();
            }
        }
    });
    std::thread::spawn(move || {
        while let Ok(event) = TrayIconEvent::receiver().recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                if panel.is_visible() {
                    panel.hide();
                } else {
                    if rect.size.height > 0 {
                        let point = rect.position.cast::<i32>();
                        let height = i32::try_from(rect.size.height).unwrap_or(0);
                        panel.set_position((
                            point.x.saturating_sub(300),
                            point.y.saturating_add(height),
                        ));
                    } else {
                        panel.place(
                            DisplayTarget::UnderPointer,
                            OverlayPlacement::Edge {
                                edge: ScreenEdge::Top,
                                margin: 32.0,
                                align: Align::End,
                                inset: 16.0,
                            },
                        );
                    }
                    panel.show();
                    panel.focus();
                }
            }
        }
    });
}
