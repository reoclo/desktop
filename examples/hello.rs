//! Rune's hello example, adapted only for formatting.
use rune::tok::{self, motion, type_};
use rune::{AppSpec, Cx, El, col, el, text};

#[derive(Default)]
struct State {
    on: bool,
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    let knob = el()
        .key("knob")
        .size(18.0, 18.0)
        .radius(9.0)
        .fill(t.knob)
        .absolute(if s.on { 18.0 } else { 2.0 }, 2.0);
    col()
        .w_full()
        .h_full()
        .center()
        .gap(16.0)
        .fill(t.bg)
        .child(
            text(if s.on { "Lights on" } else { "Lights off" })
                .font(type_::TITLE)
                .color(t.text),
        )
        .child(
            el().key("switch")
                .role("switch")
                .label("Lights")
                .checked(s.on)
                .focusable(true)
                .size(38.0, 22.0)
                .radius(11.0)
                .fill(if s.on {
                    tok::accent(0, true)
                } else {
                    t.control_off
                })
                .motion(motion::SNAPPY)
                .on_press(|s: &mut State| s.on = !s.on)
                .child(knob),
        )
}

fn main() {
    rune::run(AppSpec::<State> {
        scenes: &[("on", |s| s.on = true)],
        min_size: Some((320.0, 200.0)),
        ..AppSpec::new("Hello", (320.0, 200.0), State::default, view)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn switch_changes_state_on_runes_virtual_clock() {
        let mut rt = Runtime::new(State::default(), view, (320.0, 200.0), Env::new(true, 2.0));
        rt.frame(0.0);
        let (x, y) = rt.find("switch").unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0 / 60.0);
        assert!(rt.state.on);
    }
}
