mod components;
mod state;
mod surfaces;
mod view;

use assets::Assets;
use gpui::{App, QuitMode};
use gpui_platform::application;
use services::IpcCommand;

const SOCKET_NAME: &str = "starshell-lock";

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
        Some("lock") => Some("lock"),
        Some("quit") => Some("quit"),
        Some("--version" | "-v" | "version") => {
            println!("Starshell lock {}", env!("CARGO_PKG_VERSION"));
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

    application()
        .with_assets(Assets {})
        .with_quit_mode(QuitMode::Explicit)
        .run(|cx| {
            if let Err(error) = cx.text_system().add_fonts(assets::load_fonts()) {
                services::log_error!("Lock", "Failed to load fonts: {error}");
            }
            cx.set_global(state::LockState::new());
            surfaces::init(cx);
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
                        IpcCommand::Lock => surfaces::lock(cx),
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
        r#"Starshell lock - idle, sleep and session lock daemon

USAGE:
    starshell-lock [COMMAND]

COMMANDS:
    (no args)           Start the daemon
    lock                Lock the screen now
    quit                Stop the running daemon
    help, --help, -h    Print this help message"#
    );
}
