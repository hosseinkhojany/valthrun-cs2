// fileName: config.rs
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap,
    },
    fs::File,
    io::{
        BufReader,
        BufWriter,
    },
    path::PathBuf,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};

use anyhow::Context;
use imgui::Key;
use serde::{
    Deserialize,
    Serialize,
};
use serde_with::with_prefix;
use utils_state::{
    State,
    StateCacheType,
};


use super::{
    Color,
    EspConfig,
    EspPlayerSettings,
    EspSelector,
    EspSmoothingSettings,
    HotKey,
};

fn default_key_aimbot() -> Option<HotKey> {
    Some(imgui::Key::X.into())
}
fn default_aim_fov() -> f32 {
    100.0
}
fn default_aim_smooth() -> f32 {
    5.0
}
fn default_curve_intensity() -> f32 {
    0.4
}
fn default_aim_target() -> AimTarget {
    AimTarget::Head
}

fn bool_true() -> bool {
    true
}
fn bool_false() -> bool {
    false
}
fn default_u32<const V: u32>() -> u32 {
    V
}
fn default_i32<const V: i32>() -> i32 {
    V
}
fn default_usize<const V: usize>() -> usize {
    V
}
fn default_f32<const N: usize, const D: usize>() -> f32 {
    N as f32 / D as f32
}
fn default_color<const R: u8, const G: u8, const B: u8, const A: u8>() -> Color {
    Color::from_u8([R, G, B, A])
}

fn default_key_settings() -> HotKey {
    Key::Home.into()
}
fn default_key_trigger_bot() -> Option<HotKey> {
    Some(Key::MouseMiddle.into())
}
fn default_key_none() -> Option<HotKey> {
    None
}

fn default_esp_mode() -> KeyToggleMode {
    KeyToggleMode::AlwaysOn
}

fn default_trigger_bot_mode() -> KeyToggleMode {
    KeyToggleMode::Trigger
}

/// Numpad 1 through 9. Index 0 is numpad 1.
pub const TRIGGER_SHOT_PRESET_COUNT: usize = 9;

pub const TRIGGER_SHOT_PRESET_KEYS: [Key; TRIGGER_SHOT_PRESET_COUNT] = [
    Key::Keypad1,
    Key::Keypad2,
    Key::Keypad3,
    Key::Keypad4,
    Key::Keypad5,
    Key::Keypad6,
    Key::Keypad7,
    Key::Keypad8,
    Key::Keypad9,
];

fn default_trigger_shot_presets() -> [u32; TRIGGER_SHOT_PRESET_COUNT] {
    [100, 200, 300, 400, 500, 600, 700, 800, 900]
}

fn default_esp_configs() -> BTreeMap<String, EspConfig> {
    let mut result: BTreeMap<String, EspConfig> = Default::default();
    result.insert(
        "player.enemy".to_string(),
        EspConfig::Player(EspPlayerSettings::new(&EspSelector::PlayerTeam {
            enemy: true,
        })),
    );
    result
}

fn default_esp_configs_enabled() -> BTreeMap<String, bool> {
    let mut result: BTreeMap<String, bool> = Default::default();
    result.insert("player.enemy".to_string(), true);
    result
}


fn default_hitmarker_type() -> HitMarkerType {
    HitMarkerType::Off
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum KeyToggleMode {
    AlwaysOn,
    Toggle,
    Trigger,
    TriggerInverted,
    Off,
}

/// Body bone the aimbot pulls the crosshair toward.
///
/// Names match the CS2 player skeleton (`head_0`, `neck_0`, `spine_2`, `spine_1`).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AimTarget {
    Head,
    Neck,
    Chest,
    MiddleBody,
}

impl AimTarget {
    /// Preferred bone first, then the nearest bone in the same region.
    pub fn bone_names(self) -> &'static [&'static str] {
        match self {
            Self::Head => &["head_0"],
            Self::Neck => &["neck_0"],
            Self::Chest => &["spine_2", "spine_3"],
            Self::MiddleBody => &["spine_1", "spine_0"],
        }
    }

    /// Fraction of the player hull height above the feet when the skeleton bone is not live.
    pub fn body_height_factor(self) -> f32 {
        match self {
            Self::Head => 0.90,
            Self::Neck => 0.80,
            Self::Chest => 0.65,
            Self::MiddleBody => 0.45,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum HitMarkerType {
    //Battlefield,
    //Cod,
    Rust,
    Pat,
    Serdar,
    Off,
}



#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrenadeType {
    Smoke,
    Molotov,
    Flashbang,
    Explosive,
}

impl GrenadeType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Smoke => "Smoke",
            Self::Molotov => "Molotov",
            Self::Flashbang => "Flashbang",
            Self::Explosive => "Explosive",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrenadeSortOrder {
    Alphabetical,
    AlphabeticalReverse,
}

impl GrenadeSortOrder {
    pub fn default() -> Self {
        Self::Alphabetical
    }

    pub fn sort(&self, values: &mut Vec<&GrenadeSpotInfo>) {
        match self {
            Self::Alphabetical => {
                values.sort_unstable_by(|a, b| a.name.cmp(&b.name));
            }
            Self::AlphabeticalReverse => {
                values.sort_unstable_by(|a, b| b.name.cmp(&a.name));
            }
        }
    }
}

static GRENADE_SPOT_ID_INDEX: AtomicUsize = AtomicUsize::new(1);
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GrenadeSpotInfo {
    #[serde(skip, default = "GrenadeSpotInfo::new_id")]
    pub id: usize,
    pub grenade_types: Vec<GrenadeType>,

    pub name: String,
    pub description: String,

    /// The eye position of the player
    pub eye_position: [f32; 3],
    pub eye_direction: [f32; 2],
}
impl GrenadeSpotInfo {
    pub fn new_id() -> usize {
        GRENADE_SPOT_ID_INDEX.fetch_add(1, Ordering::Relaxed)
    }
}
#[derive(Clone, Deserialize, Serialize)]
pub struct GrenadeSettings {
    #[serde(default = "bool_true")]
    pub active: bool,

    #[serde(default = "GrenadeSortOrder::default")]
    pub ui_sort_order: GrenadeSortOrder,

    #[serde(default = "default_f32::<150, 1>")]
    pub circle_distance: f32,

    #[serde(default = "default_f32::<20, 1>")]
    pub circle_radius: f32,

    #[serde(default = "default_usize::<32>")]
    pub circle_segments: usize,

    #[serde(default = "default_f32::<1, 10>")]
    pub angle_threshold_yaw: f32,

    #[serde(default = "default_f32::<5, 10>")]
    pub angle_threshold_pitch: f32,

    #[serde(default = "default_color::<255, 255, 255, 255>")]
    pub color_position: Color,

    #[serde(default = "default_color::<0, 255, 0, 255>")]
    pub color_position_active: Color,

    #[serde(default = "default_color::<255, 0, 0, 255>")]
    pub color_angle: Color,

    #[serde(default = "default_color::<0, 255, 0, 255>")]
    pub color_angle_active: Color,

    #[serde(default)]
    pub map_spots: HashMap<String, Vec<GrenadeSpotInfo>>,

    #[serde(default = "bool_true")]
    pub grenade_background: bool,
}

with_prefix!(serde_prefix_grenade_helper "grenade_helper");

#[derive(Clone, Deserialize, Serialize)]
pub struct AppSettings {
    #[serde(default = "default_key_settings")]
    pub key_settings: HotKey,

    #[serde(default = "bool_false")]
    pub key_settings_ignore_insert_warning: bool,

    #[serde(default = "default_esp_mode")]
    pub esp_mode: KeyToggleMode,

    #[serde(default = "default_key_none")]
    pub esp_toggle: Option<HotKey>,

    #[serde(default = "default_esp_configs")]
    pub esp_settings: BTreeMap<String, EspConfig>,

    #[serde(default = "default_esp_configs_enabled")]
    pub esp_settings_enabled: BTreeMap<String, bool>,

    #[serde(default)]
    pub esp_smoothing: EspSmoothingSettings,

    #[serde(default = "bool_true")]
    pub bomb_timer: bool,

    #[serde(default = "bool_true")]
    pub bomb_label: bool,

    #[serde(default = "bool_false")]
    pub spectators_list: bool,

    #[serde(default = "bool_true")]
    pub valthrun_watermark: bool,

    #[serde(default = "default_i32::<16364>")]
    pub mouse_x_360: i32,

    #[serde(default = "default_trigger_bot_mode")]
    pub trigger_bot_mode: KeyToggleMode,

    #[serde(default = "default_key_trigger_bot")]
    pub key_trigger_bot: Option<HotKey>,

    #[serde(default = "bool_true")]
    pub trigger_bot_team_check: bool,

    #[serde(default = "default_u32::<10>")]
    pub trigger_bot_delay_min: u32,

    #[serde(default = "default_u32::<20>")]
    pub trigger_bot_delay_max: u32,

    #[serde(default = "default_u32::<400>")]
    pub trigger_bot_shot_duration: u32,

    /// Durations bound to numpad 1-9. Index 0 is numpad 1.
    #[serde(default = "default_trigger_shot_presets")]
    pub trigger_bot_shot_presets: [u32; TRIGGER_SHOT_PRESET_COUNT],

    /// Active numpad preset. Empty until a numpad key selects one.
    #[serde(default)]
    pub trigger_bot_shot_preset: Option<u8>,

    #[serde(default = "bool_false")]
    pub trigger_bot_check_target_after_delay: bool,

    #[serde(default = "bool_false")]
    pub aim_assist_recoil: bool,

    #[serde(default = "default_u32::<1>")]
    pub aim_assist_recoil_min_bullets: u32,

    #[serde(default = "bool_true")]
    pub hide_overlay_from_screen_capture: bool,

    #[serde(default = "bool_false")]
    pub render_debug_window: bool,

    #[serde(default = "bool_false")]
    pub debug_window: bool,

    #[serde(default = "bool_true")]
    pub metrics: bool,

    #[serde(default)]
    pub web_radar_url: Option<String>,

    #[serde(default = "bool_false")]
    pub web_radar_enabled: bool,

    #[serde(default = "bool_false")]
    pub web_radar_advanced_settings: bool,

    #[serde(default = "bool_true")]
    pub sniper_crosshair: bool,

    #[serde(flatten, with = "serde_prefix_grenade_helper")]
    pub grenade_helper: GrenadeSettings,

    #[serde(default)]
    pub imgui: Option<String>,

    // aimbot
    #[serde(default = "bool_false")]
    pub aim_bot_enabled: bool,

    #[serde(default = "bool_false")]
    pub aim_bot_draw_fov: bool,

    #[serde(default = "bool_true")]
    pub aim_bot_draw_pointer: bool,

    #[serde(default = "default_key_aimbot")]
    pub aim_bot_key: Option<HotKey>,

    #[serde(default = "default_aim_fov")]
    pub aim_bot_fov: f32,

    #[serde(default = "default_aim_smooth")]
    pub aim_bot_smooth: f32,

    #[serde(default = "default_curve_intensity")]
    pub aim_bot_curve: f32,

    #[serde(default = "default_aim_target")]
    pub aim_bot_target: AimTarget,

    /// Player names excluded from ESP.
    #[serde(default)]
    pub esp_disabled_players: BTreeSet<String>,

    /// Player names excluded from aimbot and trigger bot.
    #[serde(default)]
    pub aim_disabled_players: BTreeSet<String>,

    /// Old combined filter. Folded into both sets on load, then dropped from the file.
    #[serde(default, rename = "disabled_players", skip_serializing)]
    legacy_disabled_players: BTreeSet<String>,

    // hitmarker
    #[serde(default = "bool_false")]
    pub hitmarker_enabled: bool,

    #[serde(default = "default_color::<255, 255, 255, 255>")]
    pub hitmarker_color: Color,

    #[serde(default = "default_hitmarker_type")]
    pub hitmarker_type: HitMarkerType,

    // data collector
    #[serde(default = "bool_false")]
    pub collect_data: bool,

    #[serde(default = "default_u32::<10>")]
    pub collect_data_time: u32,
}

impl AppSettings {
    /// Copies a numpad preset into the live shoot duration.
    pub fn select_trigger_shot_preset(&mut self, index: usize) -> bool {
        let Some(duration) = self.trigger_bot_shot_presets.get(index).copied() else {
            return false;
        };

        let next = Some(index as u8);
        let changed =
            self.trigger_bot_shot_preset != next || self.trigger_bot_shot_duration != duration;
        self.trigger_bot_shot_preset = next;
        self.trigger_bot_shot_duration = duration;
        changed
    }

    /// ESP runs for this player only when true.
    pub fn player_esp_enabled(&self, player_name: Option<&str>) -> bool {
        Self::name_allowed(&self.esp_disabled_players, player_name)
    }

    /// Aimbot and trigger bot run for this player only when true.
    pub fn player_aim_enabled(&self, player_name: Option<&str>) -> bool {
        Self::name_allowed(&self.aim_disabled_players, player_name)
    }

    fn name_allowed(disabled: &BTreeSet<String>, player_name: Option<&str>) -> bool {
        match player_name {
            Some(name) if !name.is_empty() => !disabled.contains(name),
            _ => true,
        }
    }
}

fn migrate_legacy_player_filters(settings: &mut AppSettings) {
    if settings.legacy_disabled_players.is_empty() {
        return;
    }

    let legacy = std::mem::take(&mut settings.legacy_disabled_players);
    for name in legacy {
        settings.esp_disabled_players.insert(name.clone());
        settings.aim_disabled_players.insert(name);
    }
}

impl State for AppSettings {
    type Parameter = ();

    fn cache_type() -> StateCacheType {
        StateCacheType::Persistent
    }
}

pub fn get_settings_path() -> anyhow::Result<PathBuf> {
    let exe_file = std::env::current_exe().context("missing current exe path")?;
    let base_dir = exe_file.parent().context("could not get exe directory")?;

    Ok(base_dir.join("config.yaml"))
}

pub fn load_app_settings() -> anyhow::Result<AppSettings> {
    let config_path = get_settings_path()?;
    if !config_path.is_file() {
        log::info!(
            "App config file {} does not exist.",
            config_path.to_string_lossy()
        );
        log::info!("Using default config.");
        let mut config: AppSettings =
            serde_yaml::from_str("").context("failed to parse empty config")?;
        migrate_legacy_player_filters(&mut config);

        return Ok(config);
    }

    let config = File::open(&config_path).with_context(|| {
        format!(
            "failed to open app config at {}",
            config_path.to_string_lossy()
        )
    })?;
    let mut config = BufReader::new(config);

    let mut config: AppSettings =
        serde_yaml::from_reader(&mut config).context("failed to parse app config")?;
    migrate_legacy_player_filters(&mut config);

    log::info!("Loaded app config from {}", config_path.to_string_lossy());
    Ok(config)
}

pub fn save_app_settings(settings: &AppSettings) -> anyhow::Result<()> {
    let config_path = get_settings_path()?;
    let config = File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&config_path)
        .with_context(|| {
            format!(
                "failed to open app config at {}",
                config_path.to_string_lossy()
            )
        })?;
    let mut config = BufWriter::new(config);

    serde_yaml::to_writer(&mut config, settings).context("failed to serialize config")?;

    log::debug!("Saved app config.");
    Ok(())
}