use crate::{
    CalendarService, ClipboardService, CompositorService, ConfigService, DisplayService,
    IdleService, LauncherService, LyricsService, MprisService, NetworkService, PolkitService,
    PowerService, RecordService, ShelfService, SniHostService, SystemService, VpnService,
    wallpaper::WallpaperService,
};

#[derive(Clone)]
pub struct AppState {
    pub config: ConfigService,
    pub launcher: LauncherService,
    pub mpris: MprisService,
    pub system: SystemService,
    pub displays: DisplayService,
    pub lyrics: LyricsService,
    pub compositor: CompositorService,
    pub polkit: PolkitService,
    pub sni_host: SniHostService,
    pub wallpaper: WallpaperService,
    pub network: NetworkService,
    pub vpn: VpnService,
    pub calendar: CalendarService,
    pub power: PowerService,
    pub clipboard: ClipboardService,
    pub idle: IdleService,
    pub record: RecordService,
    pub shelf: ShelfService,
}

impl gpui::Global for AppState {}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let _guard = crate::tokio_handle().enter();
        let config = ConfigService::new();
        let sni_host = SniHostService::new();
        sni_host.start();

        let compositor = CompositorService::new();
        let wallpaper = WallpaperService::new(compositor.clone());
        let network = NetworkService::new();
        let vpn = VpnService::new();
        let calendar = CalendarService::new();
        let power = PowerService::new();
        let clipboard = ClipboardService::new();
        let mpris = MprisService::new(config.clone());
        let idle = IdleService::new(config.clone(), mpris.clone());
        let record = RecordService::new();
        let shelf = ShelfService::new();

        Self {
            config: config.clone(),
            launcher: LauncherService::new(),
            mpris,
            system: SystemService::new(),
            displays: DisplayService::new(),
            lyrics: LyricsService::new(),
            compositor,
            polkit: PolkitService::new(),
            sni_host,
            wallpaper,
            network,
            vpn,
            calendar,
            power,
            clipboard,
            idle,
            record,
            shelf,
        }
    }
}
