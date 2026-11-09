//! A bounded native probe. No daemon, preferences, or system installation.
use std::io::BufRead;
use std::sync::mpsc;
use std::time::Duration;

use rune::{App, AppSpec, Cx, El, WindowOptions, col, text};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[derive(Default)]
struct Spike {
    tray: Option<TrayIcon>,
    clicks: u32,
}

fn view(s: &Spike, _: &mut Cx<Spike>) -> El {
    let t = rune::tok::theme(true);
    col()
        .w_full()
        .h_full()
        .center()
        .gap(16.0)
        .fill(t.bg)
        .child(text("Reoclo tray spike").color(t.text))
        .child(text(format!("{} native clicks", s.clicks)).color(t.text))
}

fn main() {
    let app = App::new(AppSpec::new(
        "Reoclo tray spike",
        (340.0, 180.0),
        Spike::default,
        view,
    ))
    .windowless()
    .keep_running()
    .accessory()
    .window_options(WindowOptions {
        min_size: Some((340.0, 180.0)),
        max_size: Some((340.0, 180.0)),
        resizable: false,
        borderless: true,
        always_on_top: true,
        ..Default::default()
    })
    .on_window(|_| eprintln!("SPIKE window_created"))
    .on_quit(|s, _| {
        s.tray = None;
        eprintln!("SPIKE tray_dropped");
    });
    let handle = app.handle();
    let (stop, stopped) = mpsc::channel();
    let watchdog = handle.clone();
    let deadline = std::thread::spawn(move || {
        if stopped.recv_timeout(Duration::from_secs(90)).is_err() {
            eprintln!("SPIKE bounded_timeout");
            watchdog.quit();
        }
    });
    handle.update(|s, tx| {
        match TrayIconBuilder::new()
            .with_title("Reoclo spike")
            .with_tooltip("Temporary Reoclo tray integration probe")
            .build()
        {
            Ok(tray) => {
                eprintln!("SPIKE tray_created {:?}", tray.rect());
                s.tray = Some(tray);
            }
            Err(error) => {
                eprintln!("SPIKE tray_error {error}");
                tx.quit();
            }
        }
    });
    let events = handle.clone();
    std::thread::spawn(move || {
        while let Ok(event) = TrayIconEvent::receiver().recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let showing = events.is_visible();
                eprintln!("SPIKE native_click visible_before={showing}");
                events.update(|s, _| s.clicks += 1);
                if showing {
                    events.hide();
                } else {
                    events.set_position((600, 80));
                    events.show();
                    events.focus();
                }
            }
        }
    });
    let commands = handle.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            match line.as_deref() {
                Ok("status") => eprintln!(
                    "SPIKE visible={} error={:?}",
                    commands.is_visible(),
                    commands.window_error()
                ),
                Ok("show") => commands.open(),
                Ok("hide") => commands.hide(),
                Ok("quit") | Err(_) => {
                    commands.quit();
                    break;
                }
                Ok(_) => eprintln!("SPIKE commands: status show hide quit"),
            }
        }
    });
    app.run();
    if let Err(error) = stop.send(()) {
        eprintln!("SPIKE watchdog_already_stopped {error}");
    }
    if deadline.join().is_err() {
        eprintln!("SPIKE watchdog_panicked");
    }
    eprintln!("SPIKE stopped");
}
