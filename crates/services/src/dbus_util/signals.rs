use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::Notify;
use zbus::message::Type;
use zbus::{MatchRule, MessageStream, OwnedMatchRule};

#[derive(Clone, Copy)]
pub enum Bus {
    System,
    Session,
}

pub struct SignalRule {
    pub sender: Option<&'static str>,
    pub arg0_namespace: Option<&'static str>,
    pub interface: Option<&'static str>,
    pub member: Option<&'static str>,
    pub path: Option<&'static str>,
    pub path_namespace: Option<&'static str>,
}

impl SignalRule {
    pub const fn from(sender: &'static str) -> Self {
        Self::any().sender(sender)
    }

    pub const fn any() -> Self {
        Self {
            sender: None,
            arg0_namespace: None,
            interface: None,
            member: None,
            path: None,
            path_namespace: None,
        }
    }

    pub const fn sender(mut self, sender: &'static str) -> Self {
        self.sender = Some(sender);
        self
    }

    pub const fn arg0_namespace(mut self, namespace: &'static str) -> Self {
        self.arg0_namespace = Some(namespace);
        self
    }

    pub const fn interface(mut self, interface: &'static str) -> Self {
        self.interface = Some(interface);
        self
    }

    pub const fn member(mut self, member: &'static str) -> Self {
        self.member = Some(member);
        self
    }

    pub const fn path(mut self, path: &'static str) -> Self {
        self.path = Some(path);
        self
    }

    pub const fn path_namespace(mut self, path_namespace: &'static str) -> Self {
        self.path_namespace = Some(path_namespace);
        self
    }

    fn build(&self) -> zbus::Result<OwnedMatchRule> {
        let mut builder = MatchRule::builder().msg_type(Type::Signal);
        if let Some(sender) = self.sender {
            builder = builder.sender(sender)?;
        }
        if let Some(namespace) = self.arg0_namespace {
            builder = builder.arg0ns(namespace)?;
        }
        if let Some(interface) = self.interface {
            builder = builder.interface(interface)?;
        }
        if let Some(member) = self.member {
            builder = builder.member(member)?;
        }
        if let Some(path) = self.path {
            builder = builder.path(path)?;
        }
        if let Some(path_namespace) = self.path_namespace {
            builder = builder.path_namespace(path_namespace)?;
        }
        Ok(builder.build().into())
    }
}

/// Wakes the returned notifier whenever one of the signals arrives, so
/// services can refresh on change instead of polling.
pub fn notifier(bus: Bus, rules: Vec<SignalRule>) -> Arc<Notify> {
    let notify = Arc::new(Notify::new());
    let target = notify.clone();
    crate::spawn_tokio(async move {
        loop {
            if let Err(error) = forward(bus, &rules, &target).await {
                crate::log_warn!("D-Bus", "Signal watch stopped: {error}");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    notify
}

async fn forward(bus: Bus, rules: &[SignalRule], target: &Notify) -> zbus::Result<()> {
    let connection = match bus {
        Bus::System => super::get_shared_system_conn().await,
        Bus::Session => super::get_shared_session_conn().await,
    }
    .ok_or_else(|| zbus::Error::Failure("bus unavailable".into()))?;
    let mut streams = Vec::with_capacity(rules.len());
    for rule in rules {
        streams.push(MessageStream::for_match_rule(rule.build()?, &connection, Some(64)).await?);
    }
    let mut merged = futures::stream::select_all(streams);
    while merged.next().await.is_some() {
        target.notify_one();
    }
    Ok(())
}

/// Waits for a change or the fallback period, then lets a burst of
/// signals settle so one refresh covers all of them.
pub async fn settle(notify: &Notify, fallback: Duration) {
    tokio::select! {
        _ = notify.notified() => {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        _ = tokio::time::sleep(fallback) => {}
    }
}
