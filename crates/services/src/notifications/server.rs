use std::time::Duration;
use zbus::interface;

use super::store::NotificationStore;

pub struct NotificationServer {
    pub(crate) store: NotificationStore,
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationServer {
    pub(crate) async fn notify(
        &self,
        app_name: String,
        _replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        _hints: std::collections::HashMap<String, zbus::zvariant::Value<'_>>,
        expire_timeout: i32,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> u32 {
        self.store.add_notification_with_actions(
            (app_name, app_icon, summary, body),
            expire_timeout,
            actions,
            header.sender().map(|sender| sender.to_owned().into()),
        )
    }

    pub(crate) async fn close_notification(&self, id: u32) {
        self.store.remove_notification(id);
    }

    #[zbus(signal)]
    pub(crate) async fn notification_replied(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        id: u32,
        text: String,
    ) -> zbus::Result<()>;

    pub(crate) async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".to_string(),
            "body-markup".to_string(),
            "actions".to_string(),
            "inline-reply".to_string(),
            "icon-static".to_string(),
        ]
    }

    pub(crate) async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Starshell Notifications".to_string(),
            "Starshell".to_string(),
            "0.1.0".to_string(),
            "1.2".to_string(),
        )
    }
}

pub async fn start_notification_server() -> anyhow::Result<()> {
    let store = NotificationStore::global().clone();
    let server = NotificationServer {
        store: store.clone(),
    };

    let connection = zbus::connection::Builder::session()?
        .name("org.freedesktop.Notifications")?
        .serve_at("/org/freedesktop/Notifications", server)?
        .build()
        .await?;
    store
        .connection
        .set(connection)
        .map_err(|_| anyhow::anyhow!("Notification server connection is already initialized"))?;

    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
    }
}
