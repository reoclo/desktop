use crate::{theme::*, ui::State};
use rune::{El, col, text};
use rune_kit::{Kit, Select, SelectAppearance};

pub(super) fn choice(
    s: &State,
    key: &str,
    label: &str,
    values: &[String],
    current: &str,
) -> Select {
    let toggle = key.to_owned();
    Select::new(key, values)
        .label(label)
        .current(if current.is_empty() {
            None
        } else {
            Some(current)
        })
        .appearance(SelectAppearance::Field)
        .width(480.0)
        .open(s.open_select.as_deref() == Some(key))
        .on_toggle(move |s: &mut State| {
            s.open_select = if s.open_select.as_deref() == Some(&toggle) {
                None
            } else {
                Some(toggle.clone())
            };
        })
        .on_close(|s: &mut State| s.open_select = None)
}

pub(super) fn label(caption: &str, control: El, k: &Kit) -> El {
    col()
        .w_full()
        .gap(SM)
        .child(text(caption).font(LABEL).color(k.c.text_base))
        .child(control)
}
