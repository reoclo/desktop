use super::State;
use crate::theme::*;
use rune::{El, col, el, row, text};
use rune_kit::{IconButton, IconButtonVariant, Kit};

/// App modal shell. Rune owns focus trapping and dismissal; this header supplies heading semantics.
pub struct Dialog {
    key: String,
    title: String,
    width: f32,
    body: Vec<El>,
    close: fn(&mut State),
}

impl Dialog {
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            width: 480.0,
            body: Vec::new(),
            close: |s| s.modal = None,
        }
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    pub fn on_close(mut self, close: fn(&mut State)) -> Self {
        self.close = close;
        self
    }
    pub fn child(mut self, child: El) -> Self {
        self.body.push(child);
        self
    }
    pub fn build(self, k: &Kit, layer: &mut Vec<El>) {
        let close = self.close;
        let panel = col()
            .key(format!("{}-dialog", self.key))
            .role("dialog")
            .label(&self.title)
            .w_full()
            .max_w(self.width)
            .radius(k.radius.xl)
            .fill(k.c.layer_01)
            .border(RULE, k.c.border_base)
            .child(
                row()
                    .key("header")
                    .w_full()
                    .padding(LG)
                    .gap(SM)
                    .center_y()
                    .child(
                        text(self.title)
                            .role("heading")
                            .font(HEADING)
                            .color(k.c.text_base)
                            .grow(1.0),
                    )
                    .child(
                        IconButton::new(k.glyph.close)
                            .variant(IconButtonVariant::Ghost)
                            .label("Close")
                            .on_press(close)
                            .build(k)
                            .key("close"),
                    ),
            )
            .child(col().key("body").w_full().children(self.body))
            .token_motion(k.motion.overlay)
            .modal(true, close)
            .modal_click_outside(false);
        layer.push(
            el().key(format!("{}-scrim", self.key))
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .fill(k.c.scrim)
                .token_motion(k.motion.overlay)
                .a11y_hidden(),
        );
        layer.push(
            el().key(format!("{}-layer", self.key))
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .padding(LG)
                .center()
                .child(panel),
        );
    }
}
