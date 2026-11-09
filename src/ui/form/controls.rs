use crate::{theme::*, ui::State};
use rune::{El, Tx, col, text};
use rune_kit::{Combobox, ComboboxOption, ComboboxState, Kit, Select, SelectAppearance};

pub(super) struct Picker<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub placeholder: &'a str,
    pub search: &'a str,
    pub options: &'a [ComboboxOption],
    pub current: Option<&'a str>,
    pub disabled: bool,
}

pub(super) fn options(values: impl IntoIterator<Item = String>) -> Vec<ComboboxOption> {
    values
        .into_iter()
        .map(|value| ComboboxOption::new(value.clone(), value))
        .collect()
}

/// A dropdown that can be searched, drawn like the text inputs beside it.
pub(super) fn picker(
    p: Picker<'_>,
    state: &ComboboxState,
    access: fn(&mut State) -> &mut ComboboxState,
    choose: impl Fn(&mut State, &str, &mut Tx) + 'static,
    k: &Kit,
    layer: &mut Vec<El>,
) -> El {
    let shown = p
        .options
        .iter()
        .find(|option| Some(option.id.as_str()) == p.current)
        .map(|option| option.label.as_str());
    let trigger = Select::new::<&str>(format!("{}-value", p.key), &[])
        .label(p.label)
        .appearance(SelectAppearance::Field)
        .width(FORM_COLUMN)
        .current(shown)
        .placeholder(p.placeholder)
        .disabled(p.disabled)
        .trigger(k)
        .h(FIELD_HEIGHT)
        .border(RULE, k.c.border_strong);
    Combobox::new(p.key, state, p.options, access)
        .label(p.label)
        .current(p.current)
        .width(FORM_COLUMN)
        .trigger(trigger)
        .placeholder(p.placeholder)
        .search_placeholder(p.search)
        .empty_message("No matches")
        .disabled(p.disabled)
        .on_choose(choose)
        .build(k, layer)
}

pub(super) fn label(caption: &str, control: El, k: &Kit) -> El {
    col()
        .w(FORM_COLUMN)
        .gap(SM)
        .child(text(caption).font(LABEL).color(k.c.text_base))
        .child(control)
}
