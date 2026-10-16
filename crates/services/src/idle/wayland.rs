use std::future::poll_fn;
use std::io::ErrorKind;
use std::os::fd::AsFd;
use std::task::Poll;

use tokio::io::unix::AsyncFd;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    protocol::{wl_registry, wl_seat},
};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

use crate::config::ConfigService;

#[derive(Debug)]
pub(super) enum IdleError {
    Unsupported,
    Failed(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for IdleError {
    fn from(error: E) -> Self {
        Self::Failed(error.into())
    }
}

#[derive(Default)]
struct IdleState {
    seat: Option<wl_seat::WlSeat>,
    notifier: Option<ExtIdleNotifierV1>,
    idled: bool,
}

impl Dispatch<wl_registry::WlRegistry, ()> for IdleState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name, interface, ..
        } = event
        else {
            return;
        };
        match interface.as_str() {
            "wl_seat" if state.seat.is_none() => {
                state.seat = Some(registry.bind(name, 1, queue, ()));
            }
            "ext_idle_notifier_v1" => {
                state.notifier = Some(registry.bind(name, 1, queue, ()));
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for IdleState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for IdleState {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for IdleState {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, ext_idle_notification_v1::Event::Idled) {
            state.idled = true;
        }
    }
}

async fn flush(
    connection: &Connection,
    socket: &AsyncFd<std::os::fd::BorrowedFd<'_>>,
) -> Result<(), IdleError> {
    loop {
        match connection.flush() {
            Ok(()) => return Ok(()),
            Err(wayland_client::backend::WaylandError::Io(error))
                if error.kind() == ErrorKind::WouldBlock =>
            {
                socket.writable().await?.clear_ready();
            }
            Err(error) => return Err(error.into()),
        }
    }
}

async fn read(
    events: &mut EventQueue<IdleState>,
    socket: &AsyncFd<std::os::fd::BorrowedFd<'_>>,
) -> Result<(), IdleError> {
    poll_fn(|cx| {
        let Some(guard) = events.prepare_read() else {
            return Poll::Ready(Ok(()));
        };
        match socket.poll_read_ready(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(error)) => Poll::Ready(Err(error.into())),
            Poll::Ready(Ok(mut readiness)) => match guard.read() {
                Ok(_) => Poll::Ready(Ok(())),
                Err(wayland_client::backend::WaylandError::Io(error))
                    if error.kind() == ErrorKind::WouldBlock =>
                {
                    readiness.clear_ready();
                    Poll::Ready(Ok(()))
                }
                Err(error) => Poll::Ready(Err(error.into())),
            },
        }
    })
    .await
}

/// Reports idleness through the compositor, which also honours idle
/// inhibitors such as a playing video, and calls `on_idle` each time the
/// configured lock timeout passes without input.
pub(super) async fn watch(config: ConfigService, on_idle: impl Fn()) -> Result<(), IdleError> {
    let connection = Connection::connect_to_env()?;
    let mut events = connection.new_event_queue();
    let queue = events.handle();
    let _registry = connection.display().get_registry(&queue, ());
    let mut state = IdleState::default();
    events.roundtrip(&mut state)?;
    let (Some(notifier), Some(seat)) = (state.notifier.clone(), state.seat.clone()) else {
        return Err(IdleError::Unsupported);
    };
    let socket = AsyncFd::new(connection.as_fd())?;
    let mut changes = config.subscribe();
    let mut timeout = 0;
    let mut notification: Option<ExtIdleNotificationV1> = None;
    loop {
        let desired = config.get().lockscreen.idle_timeout;
        if desired != timeout {
            if let Some(previous) = notification.take() {
                previous.destroy();
            }
            timeout = desired;
            if timeout > 0 {
                let milliseconds = u32::try_from(timeout.saturating_mul(1000)).unwrap_or(u32::MAX);
                notification =
                    Some(notifier.get_idle_notification(milliseconds, &seat, &queue, ()));
            }
        }
        events.dispatch_pending(&mut state)?;
        if std::mem::take(&mut state.idled) {
            on_idle();
        }
        flush(&connection, &socket).await?;
        tokio::select! {
            result = read(&mut events, &socket) => result?,
            change = changes.recv() => {
                if matches!(change, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                    return Err(IdleError::Failed(anyhow::anyhow!("config updates stopped")));
                }
            }
        }
    }
}
