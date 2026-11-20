use super::*;

#[test]
fn failures_flag_every_style_and_have_a_short_solid_meter_stub() {
    let counts = Counts {
        active: 1,
        total: 3,
        failed: 1,
        pending: 2,
    };
    for style in MenuBarStyle::ALL {
        let title = menu_title(style, counts).unwrap();
        assert!(title.contains('!'));
        assert!(title.contains("(2)"));
    }
    let (rgba, width, _) = icon_rgba(MenuBarStyle::IconAndMeter, counts);
    let alpha = |x: u32, y: u32| rgba[(y * width * 4 + x * 4 + 3) as usize];
    let first = MARK_SIZE + METER_GAP + BAR_WIDTH / 2;
    let bottom = (MARK_SIZE + BAR_HEIGHT) / 2 - 1;
    assert_eq!(alpha(first, MARK_SIZE / 2), 0);
    assert_eq!(alpha(first, bottom), 255);
    assert_eq!(alpha(first - BAR_WIDTH / 2, MARK_SIZE / 2), 0);
}

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
    assert_eq!(
        menu_title(
            Icon,
            Counts {
                active: 3,
                pending: 0,
                ..Default::default()
            }
        ),
        None
    );
    assert_eq!(
        menu_title(
            IconAndName,
            Counts {
                active: 3,
                pending: 0,
                ..Default::default()
            }
        )
        .as_deref(),
        Some("Reoclo")
    );
    assert_eq!(
        menu_title(
            IconAndCount,
            Counts {
                active: 3,
                pending: 0,
                ..Default::default()
            }
        )
        .as_deref(),
        Some("3")
    );
    assert_eq!(
        menu_title(
            IconAndMeter,
            Counts {
                active: 3,
                pending: 0,
                ..Default::default()
            }
        ),
        None
    );
    assert_eq!(
        menu_title(
            Icon,
            Counts {
                active: 3,
                pending: 2,
                ..Default::default()
            }
        )
        .as_deref(),
        Some("(2)")
    );
    assert_eq!(
        menu_title(
            IconAndCount,
            Counts {
                active: 3,
                pending: 2,
                ..Default::default()
            }
        )
        .as_deref(),
        Some("3 (2)")
    );
}

#[test]
fn only_the_meter_style_widens_the_icon() {
    assert_eq!(
        icon_rgba(
            MenuBarStyle::Icon,
            Counts {
                active: 2,
                total: 3,
                ..Default::default()
            }
        )
        .1,
        MARK_SIZE
    );
    assert_eq!(
        icon_rgba(
            MenuBarStyle::IconAndMeter,
            Counts {
                active: 0,
                total: 0,
                ..Default::default()
            }
        )
        .1,
        MARK_SIZE
    );
    let (_, one, _) = icon_rgba(
        MenuBarStyle::IconAndMeter,
        Counts {
            active: 1,
            total: 1,
            ..Default::default()
        },
    );
    let (_, three, _) = icon_rgba(
        MenuBarStyle::IconAndMeter,
        Counts {
            active: 1,
            total: 3,
            ..Default::default()
        },
    );
    assert_eq!(three - one, 2 * (BAR_WIDTH + BAR_GAP));
    let (_, capped, _) = icon_rgba(
        MenuBarStyle::IconAndMeter,
        Counts {
            active: 1,
            total: 40,
            ..Default::default()
        },
    );
    assert_eq!(
        capped,
        icon_rgba(
            MenuBarStyle::IconAndMeter,
            Counts {
                active: 1,
                total: 8,
                ..Default::default()
            }
        )
        .1
    );
}

#[test]
fn a_meter_bar_is_solid_when_active_and_hollow_when_not() {
    let (rgba, width, _) = icon_rgba(
        MenuBarStyle::IconAndMeter,
        Counts {
            active: 1,
            total: 2,
            ..Default::default()
        },
    );
    let alpha = |x: u32, y: u32| rgba[(y * width * 4 + x * 4 + 3) as usize];
    let middle = MARK_SIZE / 2;
    let first = MARK_SIZE + METER_GAP + BAR_WIDTH / 2;
    let second = first + BAR_WIDTH + BAR_GAP;
    assert_eq!(alpha(first, middle), 255);
    assert_eq!(alpha(second, middle), 0);
    assert_eq!(alpha(second - BAR_WIDTH / 2, middle), 255);
}
