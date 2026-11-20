use super::{Page, State, primitives::title};
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
                    .role("heading")
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
        .max_w(CONTENT_MAX)
        .padding(if s.compact { LG } else { XL })
        .gap(if s.page == Page::Tunnels { LG } else { XL });
    body = body.child(if s.page == Page::Tunnels {
        row()
            .w_full()
            .center_y()
            .gap(LG)
            .child(
                col()
                    .grow(1.0)
                    .gap(XS)
                    .child(title("Tunnels", None, k))
                    .child(bar(k, 20.0).w(160.0)),
            )
            .child(bar(k, 28.0).w(96.0))
    } else {
        title(s.page.name(), None, k)
    });
    let mut detail_heading = col().gap(XS).child(bar(k, XL));
    if s.selected().and_then(|tunnel| tunnel.subtitle()).is_some() {
        detail_heading = detail_heading.child(bar(k, LG).w(160.0));
    }
    let layout = match s.page {
        Page::Tunnels => {
            let row_height = if s.viewport_width < NARROW_WIDTH {
                CONTROL * 2.0 + SM
            } else {
                CONTROL
            };
            let mut layout = col().gap(LG);
            if s.selected().is_some() {
                layout = layout
                    .child(detail_heading)
                    .child(bar(k, CONTROL))
                    .child(
                        col()
                            .equal_columns(if s.compact { 2 } else { 4 })
                            .w_full()
                            .gap(LG)
                            .children((0..4).map(|_| bar(k, CONTROL))),
                    )
                    .child(bar(k, CONTROL));
            }
            layout
                .child(bar(k, XL))
                .child(bar(k, GRAPH_HEIGHT))
                .child(bar(k, LG))
                .child(
                    col()
                        .gap(0.0)
                        .child(bar(k, XXL))
                        .children((0..4).map(|_| bar(k, row_height))),
                )
        }
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
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, LG)),
        Page::Settings => col()
            .gap(XL)
            .child(super::settings::updates_section::view(s, k))
            .child(bar(k, RULE))
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, RULE))
            .child(bar(k, XL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL))
            .child(bar(k, CONTROL)),
        Page::Logs => col()
            .gap(LG)
            .child(
                col()
                    .equal_columns(if s.compact { 1 } else { 2 })
                    .gap(SM)
                    .child(bar(k, CONTROL))
                    .child(bar(k, CONTROL)),
            )
            .child(bar(k, CONTROL))
            .child(bar(k, LOG_ROW_HEIGHT * 12.0)),
    };
    body = body
        .child(layout.pr(if s.page == Page::Tunnels { LG } else { 0.0 }))
        .child(
            text("Reading the local daemon.")
                .font(LABEL)
                .color(k.c.text_muted),
        );
    body
}
