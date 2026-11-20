use rune::{Family, TextStyle, Theme};
use rune_kit::{Elevation, Kit, PaletteSeed, Radius, StateColors};

pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 12.0;
pub const LG: f32 = 16.0;
pub const XL: f32 = 24.0;
pub const XXL: f32 = 32.0;
pub const RULE: f32 = 1.0;
pub const CONTROL: f32 = 36.0;
pub const LOG_ROW_HEIGHT: f32 = 24.0;
pub const GRAPH_HEIGHT: f32 = 96.0;
pub const GRAPH_STROKE: f32 = 2.0;
pub const ACTION_WIDTH: f32 = 80.0;
pub const CONTENT_MAX: f32 = 1200.0;
pub const COMPACT_WIDTH: f32 = 900.0;
pub const NARROW_WIDTH: f32 = 600.0;
pub const FACT_WIDTH: f32 = 200.0;
pub const STATE_WIDTH: f32 = 148.0;
pub const ENDPOINT_MIN: f32 = 160.0;
pub const COLUMN_MIN: f32 = 96.0;
pub const COLUMN_MAX: f32 = 220.0;
pub const MONO_ADVANCE: f32 = 7.2;
/// The widest a dropdown's list opens, Rune's popover limit; a dropdown's field matches it.
pub const MENU_WIDTH: f32 = 320.0;
/// The menu-bar panel's width, and the most height it grows to before its list scrolls.
pub const PANEL_WIDTH: f32 = 340.0;
pub const PANEL_HEIGHT: f32 = 620.0;
/// One menu-bar panel row: mark, name, endpoint and the switch or Stop on one line.
pub const PANEL_ROW: f32 = XL + XS;
/// The panel's name column, a server and its Terminal tag included, so endpoints line up down the list.
pub const PANEL_NAME: f32 = COLUMN_MIN + LG;
/// The panel's trailing slot, wide enough for a switch or a small Stop button.
pub const PANEL_ACTION: f32 = XXL + LG;
pub const TITLE: TextStyle = TextStyle::new(28.0, 800.0).family(Family::Named("Mona Sans"));
pub const HEADING: TextStyle = TextStyle::new(18.0, 600.0);
pub const BODY: TextStyle = TextStyle::new(14.0, 400.0);
pub const LABEL: TextStyle = TextStyle::new(12.0, 600.0);
pub const MONO: TextStyle = TextStyle::new(12.0, 400.0).mono();
pub const HINT: TextStyle = TextStyle::new(12.0, 400.0);
pub const WORDMARK: TextStyle = TextStyle::new(18.0, 800.0).family(Family::Named("Mona Sans"));
pub const CODE: TextStyle = TextStyle::new(18.0, 500.0).mono();
/// The first-run flow's step list, beside its page.
pub const FIRST_RUN_RAIL: f32 = 240.0;
/// The widest a first-run page's text runs, so sentences keep a readable measure.
pub const FIRST_RUN_MEASURE: f32 = 600.0;
/// One column of the two-column Add tunnel form.
pub const FORM_COLUMN: f32 = 256.0;
/// The height of a text input, which selects match so rows line up.
pub const FIELD_HEIGHT: f32 = 29.5;
pub const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/Figtree.ttf"),
    include_bytes!("../assets/fonts/JetBrainsMono.ttf"),
    include_bytes!("../assets/fonts/MonaSans.ttf"),
];

/// The Survey Sheet tokens for one appearance, from `assets/theme.json`.
pub fn tokens(dark: bool) -> Result<Theme, String> {
    Theme::from_json(include_str!("../assets/theme.json"), dark)
}

pub fn kit(dark: bool) -> Result<Kit, String> {
    let t = tokens(dark)?;
    let mut k = Kit::from_palette(PaletteSeed {
        bg: t.color("bg"),
        fg: t.color("fg"),
        accent: t.color("accent"),
    });
    k.dark = dark;
    k.c.page = t.color("bg");
    k.c.bg_base = t.color("bg");
    k.c.bg_deep = t.color("muted");
    k.c.layer_01 = t.color("popover");
    k.c.layer_02 = t.color("muted");
    k.c.layer_03 = t.color("selected");
    k.c.text_base = t.color("fg");
    k.c.text_muted = t.color("secondary");
    k.c.text_faint = t.color("secondary");
    k.c.text_strong = t.color("fg");
    k.c.text_weak = t.color("secondary");
    k.c.border_base = t.color("border");
    k.c.border_strong = t.color("input");
    k.c.border_weak = t.color("border");
    k.c.border_focus = t.color("focus");
    k.c.bg_contrast = t.color("fg");
    k.c.text_contrast = t.color("bg");
    k.c.bg_button_neutral = t.color("bg");
    k.c.bg_accent = t.color("track");
    k.c.knob = t.color("thumb");
    k.c.layer_04 = t.color("input");
    k.c.knob_on = t.color("thumb");
    k.c.depth_top = rune::Color {
        a: 0.0,
        ..t.color("fg")
    };
    k.c.depth_bottom = k.c.depth_top;
    k.c.submit_highlight = k.c.depth_top;
    k.c.surface_base = t.color("bg");
    k.c.surface_weak = t.color("muted");
    k.c.surface_hover = t.color("hover");
    k.c.hover = t.color("hover");
    k.c.pressed = t.color("selected");
    k.c.text_accent = t.color("fg");
    let status = |name| StateColors {
        bg: t.color("muted"),
        fg: t.color(name),
        border: t.color("border"),
        icon: t.color(name),
    };
    k.c.success = status("success");
    k.c.warning = status("warning");
    k.c.info = status("info");
    k.c.danger = status("danger");
    k.elevation = Elevation::default();
    // A menu opens over surfaces of its own color, so it needs an edge. A one-point rule, not a shadow.
    k.elevation.floating = vec![rune::Shadow {
        spread: RULE,
        color: t.color("input"),
        ..rune::Shadow::default()
    }];
    k.radius = Radius {
        xs: 2.0,
        sm: 2.0,
        md: 3.0,
        lg: 3.0,
        xl: 4.0,
    };
    k.ty.ui = BODY;
    k.ty.ui_medium = TextStyle::new(14.0, 500.0);
    k.ty.ui_compact = BODY;
    k.ty.body = BODY;
    k.ty.body_medium = TextStyle::new(14.0, 600.0);
    k.ty.small = TextStyle::new(12.0, 400.0);
    k.ty.small_medium = LABEL;
    k.ty.text_small = BODY;
    k.ty.text_small_medium = k.ty.body_medium;
    k.ty.text_xsmall = k.ty.small;
    k.ty.text_xsmall_medium = LABEL;
    k.ty.mono = MONO;
    Ok(k)
}
