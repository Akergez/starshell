use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationItem {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<(String, String)>,
    pub received_at: Instant,
    pub timeout: Duration,
}

#[derive(Default)]
pub(crate) struct LatestNotification {
    pub(crate) item: Option<NotificationItem>,
    pub(crate) hovered_at: Option<Instant>,
}
