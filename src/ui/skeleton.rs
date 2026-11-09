use super::{
    Page, State,
    primitives::{description, title},
};
use crate::theme::*;
use rune::{El, col, el, row, text};
use rune_kit::Kit;

fn bar(k: &Kit, height: f32) -> El {
    el().w_full()
        .h(height)
        .fill(k.c.layer_02)
        .radius(k.radius.sm)
        .a11y_hidden()
}

pub fn view(s: &State, k: &Kit) -> El {
    if s.modal.is_some() || s.panel {
        let mut body = col()
            .key("loading-state")
            .role("status")
            .label("Loading tunnel data")
            .w_full()
            .padding(LG)
            .gap(LG)
            .child(
                text("Loading tunnel data")
                    .font(HEADING)
                    .color(k.c.text_base),
            );
        for _ in 0..6 {
            body = body.child(bar(k, CONTROL));
        }
        return body;
    }
    let mut body = col()
        .key("loading-state")
        .role("status")
        .label(format!("Loading {}", s.page.name()))
        .w_full()
        .padding(XL)
        .gap(XL)
        .child(title(s.page.name(), description(s.page), k));
    let layout = match s.page {
        Page::Tunnels => col()
            .gap(LG)
            .child(
                col()
                    .h(224.0)
                    .gap(SM)
                    .child(bar(k, CONTROL))
                    .child(bar(k, CONTROL))
                    .child(bar(k, CONTROL)),
            )
            .child(bar(k, RULE))
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(
                row()
                    .gap(XL)
                    .child(bar(k, CONTROL).grow(1.0))
                    .child(bar(k, CONTROL).grow(1.0)),
            )
            .child(bar(k, CONTROL)),
        Page::Hosts => col()
            .gap(XL)
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, RULE))
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL)),
        Page::Certificates => col()
            .gap(XL)
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(bar(k, RULE))
            .child(
                row()
                    .gap(XL)
                    .child(bar(k, CONTROL).grow(1.0))
                    .child(bar(k, CONTROL).grow(1.0)),
            )
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL)),
        Page::Settings => col()
            .gap(XL)
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, RULE))
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL)),
    };
    body = body.child(layout).child(
        text("Reading the local daemon.")
            .font(LABEL)
            .color(k.c.text_muted),
    );
    body
}
