use crate::{
    model::{MenuBarStyle, TunnelState},
    ui::State,
};
use rune::{Align, AppHandle, DisplayTarget, OverlayPlacement, ScreenEdge};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

const MARK_SIZE: u32 = 44;
/// The Reoclo mark from assets/mark.svg, a pointy-top hexagon in a 45 by 50 box.
const MARK_POINTS: [(f32, f32); 6] = [
    (22.5, 0.0),
    (45.0, 12.5),
    (45.0, 37.5),
    (22.5, 50.0),
    (0.0, 37.5),
    (0.0, 12.5),
];

fn inside_mark(x: f32, y: f32) -> bool {
    (0..MARK_POINTS.len()).all(|i| {
        let (ax, ay) = MARK_POINTS[i];
        let (bx, by) = MARK_POINTS[(i + 1) % MARK_POINTS.len()];
        (bx - ax) * (y - ay) - (by - ay) * (x - ax) >= 0.0
    })
}

/// The mark as black with alpha, so macOS can tint it as a template image for
/// light and dark menu bars. Edges are antialiased by sampling each pixel.
fn mark_rgba(size: u32) -> Vec<u8> {
    const SAMPLES: u32 = 4;
    // 10% margin keeps the hexagon's points clear of the menu bar's edges.
    let scale = (size as f32 * 0.9) / 50.0;
    let (left, top) = (
        (size as f32 - 45.0 * scale) / 2.0,
        (size as f32 - 50.0 * scale) / 2.0,
    );
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let mut hits = 0u32;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let x = px as f32 + (sx as f32 + 0.5) / SAMPLES as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SAMPLES as f32;
                    if inside_mark((x - left) / scale, (y - top) / scale) {
                        hits += 1;
                    }
                }
            }
            out.extend_from_slice(&[0, 0, 0, (hits * 255 / (SAMPLES * SAMPLES)) as u8]);
        }
    }
    out
}

pub fn install(window: AppHandle<State>, panel: AppHandle<State>) {
    let fallback = window.clone();
    window.update(move |s, _| {
        let built = Icon::from_rgba(mark_rgba(MARK_SIZE), MARK_SIZE, MARK_SIZE)
            .map_err(|error| error.to_string())
            .and_then(|icon| {
                TrayIconBuilder::new()
                    .with_icon(icon)
                    .with_icon_as_template(true)
                    .with_tooltip("Reoclo local tunnels")
                    .build()
                    .map_err(|error| error.to_string())
            });
        match built {
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

const METER_GAP: u32 = 6;
const BAR_WIDTH: u32 = 8;
const BAR_GAP: u32 = 5;
const BAR_HEIGHT: u32 = 26;
const BAR_STROKE: u32 = 2;
const MAX_BARS: u32 = 8;

/// The text beside the icon. Pending approvals show in every style, because an agent is waiting.
pub fn menu_title(style: MenuBarStyle, active: usize, pending: usize) -> Option<String> {
    let approvals = (pending > 0).then(|| format!("({pending})"));
    let base = match style {
        MenuBarStyle::Icon | MenuBarStyle::IconAndMeter => None,
        MenuBarStyle::IconAndName => Some("Reoclo".to_owned()),
        MenuBarStyle::IconAndCount => Some(active.to_string()),
    };
    match (base, approvals) {
        (Some(base), Some(approvals)) => Some(format!("{base} {approvals}")),
        (base, approvals) => base.or(approvals),
    }
}

/// The icon as black with alpha. The meter adds one bar per tunnel: solid when active, outlined when not.
pub fn icon_rgba(style: MenuBarStyle, active: usize, total: usize) -> (Vec<u8>, u32, u32) {
    let mark = mark_rgba(MARK_SIZE);
    let bars = u32::try_from(total).unwrap_or(MAX_BARS).min(MAX_BARS);
    if style != MenuBarStyle::IconAndMeter || bars == 0 {
        return (mark, MARK_SIZE, MARK_SIZE);
    }
    let width = MARK_SIZE + METER_GAP + bars * BAR_WIDTH + (bars - 1) * BAR_GAP;
    let top = (MARK_SIZE - BAR_HEIGHT) / 2;
    let mut out = vec![0u8; (width * MARK_SIZE * 4) as usize];
    for y in 0..MARK_SIZE {
        let from = (y * MARK_SIZE * 4) as usize;
        let to = (y * width * 4) as usize;
        out[to..to + (MARK_SIZE * 4) as usize]
            .copy_from_slice(&mark[from..from + (MARK_SIZE * 4) as usize]);
    }
    for bar in 0..bars {
        let left = MARK_SIZE + METER_GAP + bar * (BAR_WIDTH + BAR_GAP);
        let solid = (bar as usize) < active;
        for y in top..top + BAR_HEIGHT {
            for x in left..left + BAR_WIDTH {
                let edge = x < left + BAR_STROKE
                    || x >= left + BAR_WIDTH - BAR_STROKE
                    || y < top + BAR_STROKE
                    || y >= top + BAR_HEIGHT - BAR_STROKE;
                if solid || edge {
                    out[(y * width * 4 + x * 4 + 3) as usize] = 255;
                }
            }
        }
    }
    (out, width, MARK_SIZE)
}

impl State {
    /// Bring the menu bar item in line with the tunnels. Does nothing until something shown changes.
    pub fn sync_tray(&mut self) {
        let is_active = |state: TunnelState| state == TunnelState::Active;
        let active = self
            .model
            .tunnels
            .iter()
            .filter(|t| is_active(t.state))
            .count()
            + self
                .model
                .foreground
                .iter()
                .filter(|t| is_active(t.state))
                .count();
        let total = self.model.tunnels.len() + self.model.foreground.len();
        let pending = self.model.approvals.len();
        let shown = (self.menu_bar, active, total, pending);
        if self.tray_shown == Some(shown) {
            return;
        }
        let Some(tray) = &self.tray else {
            return;
        };
        tray.set_title(menu_title(self.menu_bar, active, pending));
        let (rgba, width, height) = icon_rgba(self.menu_bar, active, total);
        let result = Icon::from_rgba(rgba, width, height)
            .map_err(|error| error.to_string())
            .and_then(|icon| {
                tray.set_icon_with_as_template(Some(icon), true)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => self.tray_shown = Some(shown),
            Err(error) => self.tray_error = Some(format!("Menu bar icon failed: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_is_a_solid_black_hexagon_on_transparency() {
        let size = MARK_SIZE as usize;
        let rgba = mark_rgba(MARK_SIZE);
        assert_eq!(rgba.len(), size * size * 4);
        let alpha = |x: usize, y: usize| rgba[(y * size + x) * 4 + 3];
        assert_eq!(alpha(size / 2, size / 2), 255);
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(size - 1, size - 1), 0);
        assert!(rgba.chunks(4).all(|p| p[..3] == [0, 0, 0]));
    }

    #[test]
    fn the_title_follows_the_style_and_always_shows_waiting_approvals() {
        use MenuBarStyle::*;
        assert_eq!(menu_title(Icon, 3, 0), None);
        assert_eq!(menu_title(IconAndName, 3, 0).as_deref(), Some("Reoclo"));
        assert_eq!(menu_title(IconAndCount, 3, 0).as_deref(), Some("3"));
        assert_eq!(menu_title(IconAndMeter, 3, 0), None);
        assert_eq!(menu_title(Icon, 3, 2).as_deref(), Some("(2)"));
        assert_eq!(menu_title(IconAndCount, 3, 2).as_deref(), Some("3 (2)"));
    }

    #[test]
    fn only_the_meter_style_widens_the_icon() {
        assert_eq!(icon_rgba(MenuBarStyle::Icon, 2, 3).1, MARK_SIZE);
        assert_eq!(icon_rgba(MenuBarStyle::IconAndMeter, 0, 0).1, MARK_SIZE);
        let (_, one, _) = icon_rgba(MenuBarStyle::IconAndMeter, 1, 1);
        let (_, three, _) = icon_rgba(MenuBarStyle::IconAndMeter, 1, 3);
        assert_eq!(three - one, 2 * (BAR_WIDTH + BAR_GAP));
        let (_, capped, _) = icon_rgba(MenuBarStyle::IconAndMeter, 1, 40);
        assert_eq!(capped, icon_rgba(MenuBarStyle::IconAndMeter, 1, 8).1);
    }

    #[test]
    fn a_meter_bar_is_solid_when_active_and_hollow_when_not() {
        let (rgba, width, _) = icon_rgba(MenuBarStyle::IconAndMeter, 1, 2);
        let alpha = |x: u32, y: u32| rgba[(y * width * 4 + x * 4 + 3) as usize];
        let middle = MARK_SIZE / 2;
        let first = MARK_SIZE + METER_GAP + BAR_WIDTH / 2;
        let second = first + BAR_WIDTH + BAR_GAP;
        assert_eq!(alpha(first, middle), 255);
        assert_eq!(alpha(second, middle), 0);
        assert_eq!(alpha(second - BAR_WIDTH / 2, middle), 255);
    }
}
