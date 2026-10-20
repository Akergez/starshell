use services::{
    ConfigService, IdleService, LyricsService, MprisService, NetworkService, PowerService,
};

#[derive(Clone)]
pub struct LockState {
    pub config: ConfigService,
    pub mpris: MprisService,
    pub lyrics: LyricsService,
    pub power: PowerService,
    pub network: NetworkService,
    _idle: IdleService,
}

impl gpui::Global for LockState {}

impl LockState {
    pub fn new() -> Self {
        let _guard = services::tokio_handle().enter();
        let config = ConfigService::new();
        let mpris = MprisService::new(config.clone());
        Self {
            _idle: IdleService::new(config.clone(), mpris.clone()),
            lyrics: LyricsService::new(),
            power: PowerService::new(),
            network: NetworkService::new(),
            config,
            mpris,
        }
    }
}
