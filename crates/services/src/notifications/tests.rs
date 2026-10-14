use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::server::NotificationServer;
use super::store::NotificationStore;

#[tokio::test]
async fn subscribers_receive_mutations_without_idle_wakes() {
    let store = NotificationStore::new();
    let mut first = store.subscribe();
    let mut second = store.subscribe();
    assert!(!first.has_changed().expect("watch open"));
    let id = add_test_notification(&store);
    first.changed().await.expect("notification added");
    second.changed().await.expect("second subscriber notified");
    assert!(!first.has_changed().expect("watch open"));
    store.set_dnd(true);
    first.changed().await.expect("DND changed");
    assert!(store.get_latest_active_notification().is_none());
    store.remove_notification(id);
    first.changed().await.expect("notification removed");
    store.clear_all_notifications();
    first.changed().await.expect("history cleared");
    assert!(
        tokio::time::timeout(Duration::from_millis(20), first.changed())
            .await
            .is_err()
    );
}

fn add_test_notification(store: &NotificationStore) -> u32 {
    store.add_notification(
        "TestApp".to_string(),
        String::new(),
        "Title".to_string(),
        String::new(),
        5000,
    )
}

fn add_reply_notification(store: &NotificationStore, key: &str, label: &str) -> u32 {
    store.add_notification_with_actions(
        (String::new(), String::new(), String::new(), String::new()),
        5000,
        vec![key.into(), label.into()],
        None,
    )
}

#[tokio::test]
async fn reply_validation_preserves_notifications() {
    let store = NotificationStore::new();
    let id = add_reply_notification(&store, "inline-reply", "Répondre");
    store.set_hovered(true);
    let original = store.get_all_notifications();
    for text in ["", " \t\n", "\u{2003}"] {
        let error = store.reply(id, text.into()).await.unwrap_err();
        assert!(error.to_string().contains("must not be blank"));
    }
    let error = store.reply(0, "Hello".into()).await.unwrap_err();
    assert!(error.to_string().contains("was not found"));
    let error = store.reply(id, "Hello".into()).await.unwrap_err();
    assert!(error.to_string().contains("has no recorded sender"));
    store
        .senders
        .lock()
        .unwrap()
        .insert(id, ":1.42".try_into().unwrap());
    let error = store.reply(id, "Hello".into()).await.unwrap_err();
    assert!(error.to_string().contains("connection is not initialized"));
    assert_eq!(store.get_all_notifications(), original);
    assert_eq!(
        store.get_latest_active_notification(),
        original.first().cloned()
    );
    assert!(
        store
            .latest_notification
            .lock()
            .unwrap()
            .hovered_at
            .is_some()
    );
}

#[tokio::test]
async fn reply_requires_exact_action_key_not_label() {
    let store = NotificationStore::new();
    for key in ["reply", "Inline-Reply", "inline-reply ", "default"] {
        let id = add_reply_notification(&store, key, "inline-reply");
        let original = store.get_all_notifications();
        let error = store.reply(id, "Hello".into()).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not advertise inline-reply")
        );
        assert_eq!(store.get_all_notifications(), original);
    }
    let id = add_test_notification(&store);
    assert!(store.reply(id, "Hello".into()).await.is_err());
}

#[tokio::test]
async fn contains_notification_tracks_close_and_clear_not_latest() {
    let store = NotificationStore::new();
    let first = add_test_notification(&store);
    let second = add_test_notification(&store);
    {
        let mut senders = store.senders.lock().unwrap();
        senders.insert(first, ":1.42".try_into().unwrap());
        senders.insert(second, ":1.43".try_into().unwrap());
    }
    assert!(store.contains_notification(first));
    assert!(store.contains_notification(second));
    assert!(!store.contains_notification(0));
    assert_eq!(store.get_latest_active_notification().unwrap().id, second);
    store.set_dnd(true);
    assert!(store.contains_notification(first));
    let server = NotificationServer {
        store: store.clone(),
    };
    server.close_notification(first).await;
    assert!(!store.contains_notification(first));
    assert!(store.contains_notification(second));
    assert!(!store.senders.lock().unwrap().contains_key(&first));
    assert!(store.senders.lock().unwrap().contains_key(&second));
    store.clear_all_notifications();
    assert!(!store.contains_notification(second));
    assert!(store.senders.lock().unwrap().is_empty());
}

#[tokio::test]
async fn inline_reply_capability_is_advertised() {
    let server = NotificationServer {
        store: NotificationStore::new(),
    };
    assert!(
        server
            .get_capabilities()
            .await
            .iter()
            .any(|capability| capability == "inline-reply")
    );
}

#[tokio::test]
#[ignore = "Requires dbus-daemon; launches a private bus, never connects to the session bus"]
async fn reply_on_private_bus() -> anyhow::Result<()> {
    use futures::StreamExt;
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut daemon = tokio::process::Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = daemon
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Missing daemon stdout"))?;
    let mut lines = BufReader::new(stdout).lines();
    let address = tokio::time::timeout(Duration::from_secs(5), lines.next_line())
        .await??
        .ok_or_else(|| anyhow::anyhow!("Missing private bus address"))?;
    tokio::time::timeout(Duration::from_secs(10), async {
        let store = NotificationStore::new();
        let connection = zbus::connection::Builder::address(address.as_str())?
            .name("org.freedesktop.Notifications")?
            .serve_at(
                "/org/freedesktop/Notifications",
                NotificationServer {
                    store: store.clone(),
                },
            )?
            .build()
            .await?;
        store
            .connection
            .set(connection.clone())
            .map_err(|_| anyhow::anyhow!("Connection already set"))?;
        let client = zbus::connection::Builder::address(address.as_str())?
            .build()
            .await?;
        let proxy = zbus::Proxy::new(
            &client,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )
        .await?;
        let introspection = zbus::fdo::IntrospectableProxy::builder(&client)
            .destination("org.freedesktop.Notifications")?
            .path("/org/freedesktop/Notifications")?
            .build()
            .await?
            .introspect()
            .await?;
        assert!(introspection.contains("<signal name=\"NotificationReplied\">"));
        let mut signals = proxy.receive_all_signals().await?;
        let observer = zbus::connection::Builder::address(address.as_str())?
            .build()
            .await?;
        let observer_proxy = zbus::Proxy::new(
            &observer,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )
        .await?;
        let mut observer_signals = observer_proxy.receive_all_signals().await?;
        let notification = (
            "TestApp",
            0_u32,
            "",
            "Title",
            "Body",
            vec!["inline-reply", "Répondre"],
            HashMap::<String, zbus::zvariant::Value<'_>>::new(),
            5000_i32,
        );
        let missing_sender = add_reply_notification(&store, "inline-reply", "Reply");
        let error = store
            .reply(missing_sender, "Private".into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("has no recorded sender"));
        assert!(store.contains_notification(missing_sender));
        store.remove_notification(missing_sender);
        let id: u32 = proxy.call("Notify", &notification).await?;
        assert_eq!(
            store
                .senders
                .lock()
                .unwrap()
                .get(&id)
                .map(|sender| sender.as_str()),
            client.unique_name().map(|name| name.as_str())
        );
        store.set_hovered(true);
        let text = "  Bonjour 🌍\n".to_string();
        store.reply(id, text.clone()).await?;
        let message = signals
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("Signal stream ended"))?;
        assert_eq!(
            message.header().member().map(|member| member.as_str()),
            Some("NotificationReplied")
        );
        assert_eq!(message.body().deserialize::<(u32, String)>()?, (id, text));
        assert_eq!(
            message.header().sender().map(|name| name.as_str()),
            connection.unique_name().map(|name| name.as_str())
        );
        assert_eq!(
            message.header().destination().map(|name| name.as_str()),
            client.unique_name().map(|name| name.as_str())
        );
        assert!(!store.contains_notification(id));
        assert!(store.senders.lock().unwrap().is_empty());
        assert!(store.get_all_notifications().is_empty());
        assert!(store.get_latest_active_notification().is_none());
        assert!(
            store
                .latest_notification
                .lock()
                .unwrap()
                .hovered_at
                .is_none()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), signals.next())
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), observer_signals.next())
                .await
                .is_err()
        );
        let closed_id: u32 = proxy.call("Notify", &notification).await?;
        let id: u32 = proxy.call("Notify", &notification).await?;
        assert!(store.contains_notification(closed_id));
        assert!(store.contains_notification(id));
        let _: () = proxy.call("CloseNotification", &(closed_id,)).await?;
        assert!(!store.contains_notification(closed_id));
        assert!(!store.senders.lock().unwrap().contains_key(&closed_id));
        assert!(store.contains_notification(id));
        store.set_hovered(true);
        let original = store.get_all_notifications();
        connection.close().await?;
        assert!(store.reply(id, "Not sent".into()).await.is_err());
        assert!(store.senders.lock().unwrap().contains_key(&id));
        assert_eq!(store.get_all_notifications(), original);
        assert_eq!(
            store.get_latest_active_notification(),
            original.first().cloned()
        );
        assert!(
            store
                .latest_notification
                .lock()
                .unwrap()
                .hovered_at
                .is_some()
        );
        Ok::<(), anyhow::Error>(())
    })
    .await??;
    daemon.kill().await?;
    daemon.wait().await?;
    Ok(())
}

#[test]
fn actions_are_published_as_pairs() {
    let store = NotificationStore::new();
    let id = store.add_notification_with_actions(
        (String::new(), String::new(), String::new(), String::new()),
        5000,
        vec![
            "default".into(),
            "Open".into(),
            "reply".into(),
            "Reply".into(),
            "unpaired".into(),
        ],
        None,
    );
    let latest = store.get_latest_active_notification().unwrap();
    assert_eq!(latest.id, id);
    assert_eq!(
        latest.actions,
        vec![
            ("default".into(), "Open".into()),
            ("reply".into(), "Reply".into())
        ]
    );
    assert_eq!(store.get_all_notifications(), vec![latest]);
    store.invoke_action(id, "missing".into());
    store.invoke_action(id, "default".into());
    assert_eq!(store.get_all_notifications().len(), 1);
    let id = add_test_notification(&store);
    assert_eq!(store.get_latest_active_notification().unwrap().id, id);
    assert!(
        store
            .get_latest_active_notification()
            .unwrap()
            .actions
            .is_empty()
    );
}

#[test]
fn hover_preserves_remaining_timeout() {
    let store = NotificationStore::new();
    add_test_notification(&store);
    let received_at = Instant::now() - Duration::from_secs(20);
    store
        .latest_notification
        .lock()
        .unwrap()
        .item
        .as_mut()
        .unwrap()
        .received_at = received_at;
    let entered_at = received_at + Duration::from_secs(2);
    store.set_hovered_at(true, entered_at);
    store.set_hovered_at(true, entered_at + Duration::from_secs(1));
    assert!(store.get_latest_active_notification().is_some());
    let exited_at = entered_at + Duration::from_secs(10);
    store.set_hovered_at(false, exited_at);
    let latest = store.latest_notification.lock().unwrap();
    let item = latest.item.as_ref().unwrap();
    assert_eq!(item.received_at, received_at + Duration::from_secs(10));
    assert_eq!(
        item.timeout - exited_at.duration_since(item.received_at),
        Duration::from_secs(3)
    );
    assert!(latest.hovered_at.is_none());
    drop(latest);
    assert!(store.get_latest_active_notification().is_none());
}

#[test]
fn replacement_does_not_inherit_hover_and_expired_items_stay_expired() {
    let store = NotificationStore::new();
    add_test_notification(&store);
    store.set_hovered(true);
    let id = add_test_notification(&store);
    {
        let mut latest = store.latest_notification.lock().unwrap();
        assert!(latest.hovered_at.is_none());
        assert_eq!(latest.item.as_ref().unwrap().id, id);
        latest.item.as_mut().unwrap().received_at = Instant::now() - Duration::from_secs(10);
    }
    store.set_hovered(true);
    assert!(store.get_latest_active_notification().is_none());
    assert!(
        store
            .latest_notification
            .lock()
            .unwrap()
            .hovered_at
            .is_none()
    );
}

#[tokio::test]
async fn close_and_clear_reset_hover() {
    let store = NotificationStore::new();
    let first = add_test_notification(&store);
    let second = add_test_notification(&store);
    store.set_hovered(true);
    let server = NotificationServer {
        store: store.clone(),
    };
    server.close_notification(first).await;
    assert_eq!(store.get_latest_active_notification().unwrap().id, second);
    assert!(
        store
            .latest_notification
            .lock()
            .unwrap()
            .hovered_at
            .is_some()
    );
    server.close_notification(second).await;
    assert!(store.get_all_notifications().is_empty());
    assert!(store.get_latest_active_notification().is_none());
    assert!(
        store
            .latest_notification
            .lock()
            .unwrap()
            .hovered_at
            .is_none()
    );
    add_test_notification(&store);
    store.set_hovered(true);
    store.clear_all_notifications();
    assert!(store.get_all_notifications().is_empty());
    assert!(store.get_latest_active_notification().is_none());
    assert!(
        store
            .latest_notification
            .lock()
            .unwrap()
            .hovered_at
            .is_none()
    );
}

#[test]
fn hover_capped_at_maximum_safety_duration() {
    let store = NotificationStore::new();
    add_test_notification(&store);
    let now = Instant::now();
    let received_at = now - Duration::from_secs(10);
    store
        .latest_notification
        .lock()
        .unwrap()
        .item
        .as_mut()
        .unwrap()
        .received_at = received_at;
    // Hover started 65 seconds ago (exceeding 60s safety cap)
    let entered_at = now - Duration::from_secs(65);
    store.set_hovered_at(true, entered_at);
    assert!(store.get_latest_active_notification().is_none());
}

#[test]
fn test_dnd_toggle_and_suppression() {
    let store = NotificationStore::new();
    assert!(!store.is_dnd_enabled());

    store.add_notification(
        "TestApp".to_string(),
        "".to_string(),
        "Title".to_string(),
        "Body".to_string(),
        5000,
    );
    assert!(store.get_latest_active_notification().is_some());
    assert_eq!(store.get_all_notifications().len(), 1);

    store.toggle_dnd();
    assert!(store.is_dnd_enabled());
    assert!(store.get_latest_active_notification().is_none());

    store.add_notification(
        "TestApp2".to_string(),
        "".to_string(),
        "Title2".to_string(),
        "Body2".to_string(),
        5000,
    );
    assert!(store.get_latest_active_notification().is_none());
    assert_eq!(store.get_all_notifications().len(), 2);

    store.toggle_dnd();
    assert!(!store.is_dnd_enabled());
}
