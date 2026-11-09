use reoclo_desktop::{
    connection::Connection,
    control::Client,
    fake::FakeDaemon,
    system::{FakeRunner, MacRunner, SystemRunner},
    theme, tray,
    ui::{Page, State, view},
};
use rune::{App, AppSpec, Key, KeyEvent, Tx, WindowOptions, WindowSpec};
use std::path::PathBuf;

fn spec() -> AppSpec<State> {
    AppSpec {
        fonts: theme::FONTS,
        families: ("Figtree", "JetBrains Mono"),
        min_size: Some((760.0, 620.0)),
        on_key: Some(keys),
        ..AppSpec::new("Reoclo", (1100.0, 760.0), State::default, view)
    }
}

fn keys(s: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    match event.key {
        Key::Command('q') => tx.quit(),
        Key::Command(',') => s.page = Page::Settings,
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
        App::new(spec).init(move || state).args(args).run();
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
    let mut app = App::new(spec())
        .init(move || State {
            start_at_login: login_enabled,
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
        let cli = executable
            .parent()
            .ok_or("Missing executable directory")?
            .join("../Resources/reoclo");
        Box::new(MacRunner {
            cli,
            desktop: executable,
            launch_agent,
        })
    };
    let system = reoclo_desktop::system::worker::start(runner, handles.clone());
    for handle in &handles {
        let requests = requests.clone();
        let system = system.clone();
        let main = main.clone();
        handle.update(move |s, _| {
            s.sender = Some(requests);
            s.system_runner = Some(system);
            s.window_handle = Some(main);
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
