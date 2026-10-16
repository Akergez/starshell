pub mod applets;
pub mod bar;
pub mod launcher;
pub mod lockscreen;
pub mod notifications;
pub mod panel;
pub mod polkit;
pub mod settings;
pub mod shell;

use assets::Assets;
use gpui_platform::application;

fn main() -> std::io::Result<()> {
    run_with_runtime(run_application)
}

fn run_with_runtime(run: impl FnOnce()) -> std::io::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _runtime_context = runtime.enter();
    run();
    Ok(())
}

fn run_application() {
    services::init_tokio_handle(tokio::runtime::Handle::current());

    #[cfg(not(target_os = "linux"))]
    compile_error!("This application is only supported on Linux.");

    if let Some(home) = dirs::home_dir() {
        let _ = std::env::set_current_dir(home);
    }

    services::init_logger();

    let args: Vec<String> = std::env::args().collect();
    let cmd_arg = if args.len() > 1 {
        Some(args[1..].join(" "))
    } else {
        None
    };

    if let Some(ref raw) = cmd_arg {
        let normalized = raw.trim().to_lowercase();
        if normalized == "--version" || normalized == "-v" || normalized == "version" {
            println!("Starshell {}", env!("CARGO_PKG_VERSION"));
            return;
        }

        if normalized == "--help" || normalized == "-h" || normalized == "help" {
            print_help();
            return;
        }

        if services::decode_command(raw).is_none() {
            eprintln!("Error: Unknown command '{raw}'.");
            eprintln!("Run 'starshell --help' for a list of available commands.");
            std::process::exit(1);
        }
    }

    let ipc_subscriber = match services::IpcSubscriber::init(cmd_arg.as_deref()) {
        Some(sub) => sub,
        None => return,
    };

    services::spawn_tokio(async {
        if let Err(err) = services::start_notification_server().await {
            eprintln!("D-Bus Notification Server warning: {err}");
        }
    });

    let app = application().with_assets(Assets {});

    app.run(|cx| {
        let font_data = assets::load_fonts();
        if let Err(err) = cx.text_system().add_fonts(font_data) {
            eprintln!("Failed to load fonts: {err}");
        }

        let app_state = services::AppState::new();
        cx.set_global(app_state);

        let ui_config = cx.global::<services::AppState>().config.get().ui.clone();
        cx.set_reduce_motion(ui_config.reduce_motion);
        let mut theme_manager = ui::theme::theme_manager::ThemeManager::new();
        theme_manager.current_theme = theme_manager
            .selected_theme
            .clone()
            .with_mode(ui_config.dark_mode);
        cx.set_global(theme_manager.current_theme.clone());
        cx.set_global(theme_manager);
        settings::appearance::initialize(cx);

        shell::state::ShellState::init(cx);
        shell::Shell::init(cx);
        polkit::PolkitAgent::init(cx);
        notifications::popup::init(cx);
        shell::ipc::start(cx);
        ipc_subscriber.start();
    });
}

fn print_help() {
    println!(
        r#"Starshell - bar, control center and launcher for Wayland

USAGE:
    starshell [COMMAND]

COMMANDS:
    (no args)           Start the Starshell daemon
    toggle-launcher     Toggle the launcher (alias: launcher)
    toggle-dashboard    Toggle the control center (alias: dashboard)
    toggle-audio        Toggle the audio applet (alias: audio)
    toggle-displays     Toggle the displays applet (alias: displays)
    toggle-media        Toggle the media player applet (alias: media)
    toggle-calendar     Toggle the calendar applet (alias: calendar)
    toggle-system       Toggle the system applet (alias: system)
    toggle-notification Toggle the notification center (alias: notifications)
    toggle-clipboard    Toggle clipboard history (aliases: clipboard, clip)
    toggle-emoji        Toggle the emoji picker (alias: emoji)
    toggle-shelf        Toggle the file shelf (alias: shelf)
    toggle-record       Toggle the screen recorder (alias: record)
    toggle-themes       Toggle the theme selector (aliases: themes, select-theme)
    toggle-wallpapers   Toggle the wallpaper selector (aliases: wallpapers, select-wallpaper)
    show-launcher       Show the launcher
    show-dashboard      Show the control center
    show-notification   Show the notification center
    show-clipboard      Show clipboard history
    show-themes         Show the theme selector
    show-wallpapers     Show the wallpaper selector
    lock                Lock the screen (alias: lockscreen)
    terminal            Launch the default terminal (alias: term)
    browser             Launch the default browser (alias: web)
    editor              Launch the default editor (alias: edit)
    files               Launch the default file manager
    settings            Open settings (aliases: config, preferences)
    hide                Close the open applet (alias: close)
    quit                Stop the running daemon (alias: exit)
    ping                Check whether the daemon is running
    help, --help, -h    Print this help message"#
    );
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::task::{Context, Poll, Wake};

    #[derive(Default)]
    struct WakeCounter(AtomicUsize);

    impl Wake for WakeCounter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn poll_foreign_executor_channel() -> (usize, usize) {
        let counter = Arc::new(WakeCounter::default());
        let waker = counter.clone().into();
        let mut context = Context::from_waker(&waker);
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        for value in 0..512 {
            sender.send(value).expect("enqueue message");
        }

        let mut received = 0;
        for _ in 0..1024 {
            if let Poll::Ready(Some(_)) = receiver.poll_recv(&mut context) {
                received += 1;
            }
        }
        (received, counter.0.load(Ordering::Relaxed))
    }

    #[test]
    fn nested_event_loop_exhausts_tokio_budget_and_self_wakes() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("runtime");
        let (received, wakes) = runtime.block_on(async { poll_foreign_executor_channel() });
        assert!(
            received < 512,
            "nested executor unexpectedly drained its channel"
        );
        assert!(
            wakes > 512,
            "expected repeated wakes without channel progress"
        );
        eprintln!("nested event loop: {received} messages received, {wakes} self-wakes");
    }

    #[test]
    fn application_runtime_keeps_channels_idle_without_starving_services() {
        super::run_with_runtime(|| {
            let (received, wakes) = poll_foreign_executor_channel();
            assert_eq!(received, 512);
            assert_eq!(wakes, 0, "an idle receiver must not reschedule itself");

            let (sender, receiver) = std::sync::mpsc::channel();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                sender.send(()).expect("report timer completion");
            });
            receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .expect("services must progress while the main thread runs GPUI");
        })
        .expect("application runtime");
    }
}
