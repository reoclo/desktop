use reoclo_desktop::{
    cli::{Cli, Locator, MIN_VERSION},
    connection::Connection,
    control::Client,
    fake::FakeDaemon,
    system::{FakeRunner, MacRunner, SystemRunner},
    theme, tray,
    ui::{Page, State, ViewState, view},
};
use rune::{App, AppSpec, Key, KeyEvent, Titlebar, Tx, WindowOptions, WindowSpec};
use std::path::PathBuf;

/// The toolbar's height, so macOS centers the window controls on the sidebar toggle.
const TITLEBAR: Titlebar = Titlebar::new(48.0);

fn spec() -> AppSpec<State> {
    AppSpec {
        fonts: theme::FONTS,
        families: ("Figtree", "JetBrains Mono"),
        min_size: Some((760.0, 620.0)),
        on_key: Some(keys),
        poll: Some(State::poll),
        ..AppSpec::new("Reoclo", (1100.0, 760.0), State::default, view)
    }
}

fn keys(s: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    match event.key {
        Key::Command('q') => tx.quit(),
        Key::Command(',') => s.page = Page::Settings,
        Key::Command('n')
            if s.modal.is_none() && matches!(s.phase, ViewState::Loaded | ViewState::Empty) =>
        {
            s.begin_add()
        }
        Key::Command(digit @ '1'..='4') if s.modal.is_none() => {
            s.show_page(Page::ALL[digit as usize - '1' as usize]);
        }
        Key::Command('w') => tx.close_window(),
        _ => return false,
    }
    true
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut screen = "tunnels".to_owned();
    let mut case = "loaded".to_owned();
    let mut args = Vec::new();
    let mut fixture = false;
    let mut demo = false;
    let mut socket = None;
    let mut reduced_motion = false;
    let mut input = std::env::args().skip(1);
    while let Some(arg) = input.next() {
        match arg.as_str() {
            "--screen" => screen = input.next().ok_or("Missing screen")?,
            "--state" => case = input.next().ok_or("Missing state")?,
            "--fixture" => fixture = true,
            "--demo" => demo = true,
            "--socket" => socket = Some(PathBuf::from(input.next().ok_or("Missing socket path")?)),
            "--reduced-motion" => reduced_motion = true,
            _ => args.push(arg),
        }
    }
    if fixture {
        let mut state = State::fixture(&screen, &case)?;
        state.reduced_motion = reduced_motion;
        let mut spec = spec();
        if screen == "panel" {
            spec.size = (340.0, 620.0);
            spec.min_size = Some(spec.size);
        }
        let app = App::new(spec).init(move || state).args(args);
        if screen == "panel" {
            app.run();
        } else {
            app.titlebar(TITLEBAR).run();
        }
        return Ok(());
    }
    let home = PathBuf::from(std::env::var("HOME")?);
    let fake = if demo {
        let run = std::env::current_dir()?.join(".cache/run");
        std::fs::create_dir_all(&run)?;
        Some(FakeDaemon::start(
            run.join(format!("demo-{}.sock", std::process::id())),
        )?)
    } else {
        None
    };
    let socket = fake
        .as_ref()
        .map(|f| f.path().to_owned())
        .or(socket)
        .or_else(|| std::env::var_os("REOCLO_CONTROL_SOCKET").map(PathBuf::from))
        .unwrap_or_else(|| {
            let config = std::env::var_os("REOCLO_CONFIG_DIR").map(PathBuf::from);
            let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
            reoclo_desktop::paths::socket(&home, config.as_deref(), xdg.as_deref())
        });
    let executable = std::env::current_exe()?;
    let launch_agent = home.join("Library/LaunchAgents/com.reoclo.desktop.plist");
    let login_enabled = if demo { false } else { launch_agent.exists() };
    if login_enabled
        && executable
            .to_string_lossy()
            .contains(".app/Contents/MacOS/")
    {
        // A login item from an older build can point at an executable since renamed.
        let current = reoclo_desktop::system::launch_agent(&executable.to_string_lossy());
        if std::fs::read_to_string(&launch_agent).ok().as_deref() != Some(current.as_str()) {
            std::fs::write(&launch_agent, current)?;
        }
    }
    let preferences_path =
        (!demo).then(|| home.join("Library/Application Support/Reoclo/preferences.json"));
    let preferences = preferences_path
        .as_deref()
        .map(reoclo_desktop::ui::read_preferences)
        .unwrap_or_default();
    let daemon_plist = home.join("Library/LaunchAgents/com.reoclo.tunneld.plist");
    let service_plist = (!demo).then(|| daemon_plist.clone());
    let service_installed = demo || service_plist.as_ref().is_some_and(|plist| plist.exists());
    let onboarded = demo || preferences.onboarded;
    let mut app = App::new(spec())
        .titlebar(TITLEBAR)
        .init(move || State {
            start_at_login: login_enabled,
            menu_bar: preferences.menu_bar,
            preferences_path,
            onboarded,
            modal: (!onboarded).then(|| "get-started".to_owned()),
            service_plist,
            service_installed,
            ..State::default()
        })
        .args(args)
        .windowless()
        .keep_running()
        .accessory()
        .standard_menus()
        .on_close(|s, tx| {
            if s.tray_error.is_some() {
                tx.quit();
            }
        })
        .on_quit(|s, _| s.tray = None)
        .on_open_url(|s, url, _| {
            match url {
                "reoclo://tunnels" => s.page = Page::Tunnels,
                "reoclo://hosts" => s.page = Page::Hosts,
                "reoclo://certificates" => s.page = Page::Certificates,
                "reoclo://settings" => s.page = Page::Settings,
                url if reoclo_desktop::link::is_open_link(url) => s.open_link(url),
                _ => s.notice = Some("This Reoclo link is not supported.".into()),
            }
            if let Some(handle) = &s.window_handle {
                handle.open();
                handle.set_dock_icon(true);
            }
        });
    let main = app.handle();
    let mut panel_spec = spec();
    panel_spec.size = (340.0, 620.0);
    panel_spec.min_size = Some(panel_spec.size);
    let panel_app = App::new(panel_spec)
        .init(|| State {
            panel: true,
            ..State::default()
        })
        .windowless();
    let (panel, _) = app.window(
        WindowSpec::from_app("panel", panel_app).options(WindowOptions {
            min_size: Some((340.0, 400.0)),
            max_size: Some((340.0, 760.0)),
            borderless: true,
            always_on_top: true,
            resizable: false,
            ..Default::default()
        }),
    );
    let handles = vec![main.clone(), panel.clone()];
    let (_connection, requests) = Connection::start(Client::new(socket), handles.clone());
    let runner: Box<dyn SystemRunner> = if demo {
        Box::new(FakeRunner)
    } else {
        Box::new(MacRunner {
            desktop: executable.clone(),
            launch_agent,
            service_plist: daemon_plist.clone(),
        })
    };
    let system = reoclo_desktop::system::worker::start(runner, handles.clone());
    let finder = (!demo).then(|| {
        reoclo_desktop::system::worker::find_cli(
            Locator {
                override_path: std::env::var_os("REOCLO_CLI").map(PathBuf::from),
                service_plist: Some(daemon_plist),
                home: home.clone(),
                bundled: executable
                    .parent()
                    .map(|dir| dir.join("../Resources/reoclo")),
            },
            handles.clone(),
        )
    });
    for (index, handle) in handles.iter().enumerate() {
        let requests = requests.clone();
        let system = system.clone();
        let main = main.clone();
        // Only the main window searches; the result reaches both windows.
        let finder = if index == 0 { finder.clone() } else { None };
        handle.update(move |s, _| {
            s.sender = Some(requests);
            s.system_runner = Some(system);
            s.window_handle = Some(main);
            if demo {
                s.cli = Some(Cli::Ready {
                    path: PathBuf::from("reoclo"),
                    version: MIN_VERSION.into(),
                    bundled: true,
                });
            }
            s.cli_finder = finder;
            s.find_cli(true);
        });
    }
    tray::install(main.clone(), panel);
    if demo {
        main.open();
    }
    app.run();
    drop(fake);
    Ok(())
}
