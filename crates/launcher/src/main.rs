mod view;
mod window;

use assets::Assets;
use gpui::App;
use gpui_platform::application;
use services::IpcCommand;

const SOCKET_NAME: &str = "starshell-launcher";

fn main() -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _runtime_context = runtime.enter();
    run();
    Ok(())
}

fn run() {
    services::init_tokio_handle(tokio::runtime::Handle::current());
    services::init_logger();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match args.first().map(String::as_str) {
        None => None,
        Some("toggle") => Some("toggle-launcher"),
        Some("show") => Some("show-launcher"),
        Some("hide") => Some("hide"),
        Some("quit") => Some("quit"),
        Some("--version" | "-v" | "version") => {
            println!("Starshell launcher {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--help" | "-h" | "help") => {
            print_help();
            return;
        }
        Some(other) => {
            eprintln!("Error: Unknown command '{other}'.");
            print_help();
            std::process::exit(1);
        }
    };

    let Some(ipc_subscriber) = services::IpcSubscriber::init_named(SOCKET_NAME, command) else {
        return;
    };

    application().with_assets(Assets {}).run(|cx| {
        if let Err(error) = cx.text_system().add_fonts(assets::load_fonts()) {
            services::log_error!("Launcher", "Failed to load fonts: {error}");
        }
        cx.set_global(ui::theme::theme_manager::ThemeManager::load_current());
        window::init(cx);
        handle_commands(cx);
        ipc_subscriber.start();
    });
}

fn handle_commands(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            services::wait_for_ipc_command().await;
            cx.update(|cx| {
                while let Some(command) = services::pop_ipc_command() {
                    match command {
                        IpcCommand::ToggleLauncher => window::toggle(cx),
                        IpcCommand::ShowLauncher => window::show(cx),
                        IpcCommand::Hide => window::close(cx),
                        IpcCommand::Quit => cx.quit(),
                        _ => {}
                    }
                }
            });
        }
    })
    .detach();
}

fn print_help() {
    println!(
        r#"Starshell launcher - application launcher backed by pop-launcher

USAGE:
    starshell-launcher [COMMAND]

COMMANDS:
    (no args)           Start the daemon
    toggle              Open or close the launcher
    show                Open the launcher
    hide                Close the launcher
    quit                Stop the running daemon
    help, --help, -h    Print this help message"#
    );
}
