mod components;
mod state;
mod surfaces;
mod view;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use assets::Assets;
use gpui::{App, QuitMode};
use gpui_platform::application;
use services::IpcCommand;

const SOCKET_NAME: &str = "starshell-lock";
const SUPERVISED_ENV: &str = "STARSHELL_LOCK_SUPERVISED";
const RESTART_DELAY: Duration = Duration::from_secs(1);
const QUICK_CRASH_WINDOW: Duration = Duration::from_secs(10);
const MAX_QUICK_CRASHES: u32 = 5;

fn main() -> std::io::Result<()> {
    if std::env::args().len() == 1 && std::env::var_os(SUPERVISED_ENV).is_none() {
        return supervise();
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _runtime_context = runtime.enter();
    run();
    Ok(())
}

// A lock client that dies leaves the compositor locked with nothing to type the password into,
// so the daemon is restarted and locks again on its own.
fn supervise() -> std::io::Result<()> {
    services::init_logger();
    let program = std::env::args_os()
        .next()
        .map_or_else(std::env::current_exe, |program| Ok(program.into()))?;
    let mut quick_crashes = 0;
    loop {
        let started = Instant::now();
        let status = Command::new(&program).env(SUPERVISED_ENV, "1").status()?;
        if status.success() || !compositor_running() {
            return Ok(());
        }
        quick_crashes = if started.elapsed() < QUICK_CRASH_WINDOW {
            quick_crashes + 1
        } else {
            0
        };
        if quick_crashes >= MAX_QUICK_CRASHES {
            services::log_error!("Lock", "The daemon keeps crashing ({status}), giving up");
            return Err(std::io::Error::other("starshell-lock keeps crashing"));
        }
        services::log_error!("Lock", "The daemon exited ({status}), restarting");
        std::thread::sleep(RESTART_DELAY);
    }
}

fn compositor_running() -> bool {
    let (Some(runtime_dir), Some(display)) = (
        std::env::var_os("XDG_RUNTIME_DIR"),
        std::env::var_os("WAYLAND_DISPLAY"),
    ) else {
        return false;
    };
    Path::new(&runtime_dir).join(display).exists()
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
