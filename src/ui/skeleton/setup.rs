use super::bar;
use crate::{theme::*, ui::State};
use rune::{El, col, el, row};
use rune_kit::Kit;

/// The first-run window before the daemon has answered: the rail and one screen's worth of bars,
/// on the same measure as the screens, so nothing shifts when the first screen arrives.
pub fn first_run(k: &Kit) -> El {
    let rail = col()
        .w(FIRST_RUN_RAIL)
        .h_full()
        .fill(k.c.bg_deep)
        .px(MD)
        .pt(XXL + LG)
        .pb(LG)
        .gap(XL)
        .child(bar(k, LG).w(120.0).mx(SM))
        .child(
            col()
                .w_full()
                .gap(XS)
                .children((0..4).map(|_| bar(k, CONTROL * 2.0))),
        );
    let content = col()
        .grow(1.0)
        .min_w(0.0)
        .max_w(FIRST_RUN_MEASURE)
        .gap(XL)
        .child(bar(k, XL).w(280.0))
        .child(
            col()
                .w_full()
                .gap(SM)
                .child(bar(k, LG))
                .child(bar(k, LG).w(360.0)),
        )
        .child(bar(k, CONTROL * 2.0))
        .child(bar(k, CONTROL * 2.0))
        .child(bar(k, CONTROL));
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .child(
            col()
                .w_full()
                .grow(1.0)
                .px(XXL)
                .pt(XXL + LG)
                .pb(XL)
                .child(content),
        )
        .child(el().w_full().h(RULE).fill(k.c.border_base))
        .child(
            row()
                .w_full()
                .px(XXL)
                .py(LG)
                .justify_end()
                .child(bar(k, CONTROL).w(120.0)),
        );
    row()
        .key("loading-state")
        .role("status")
        .label("Loading setup")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .on_layout(|s: &mut State, [_, _, width, _]| s.viewport_width = width)
        .child(rail)
        .child(el().w(RULE).h_full().fill(k.c.border_base))
        .child(main)
}
