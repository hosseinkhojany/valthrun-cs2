use std::time::{
    Duration,
    Instant,
};

use cs2::{
    MouseState,
    StateCS2Memory,
    StateLocalPlayerController,
    StatePawnInfo,
};
use cs2_schema_cutl::EntityHandle;
use cs2_schema_generated::cs2::client::{
    C_BaseEntity,
    C_CSPlayerPawn,
};
use obfstr::obfstr;
use overlay::UnicodeTextRenderer;
use rand::{
    distributions::Uniform,
    prelude::Distribution,
};
use utils_state::StateRegistry;

use super::Enhancement;
use crate::{
    settings::AppSettings,
    view::{
        KeyToggle,
        StateLocalCrosshair,
    },
    UpdateContext,
};

#[derive(Clone, Copy)]
enum TriggerState {
    Idle,
    Pending { delay: u32, timestamp: Instant },
    Cooldown { until: Instant },
}

pub struct TriggerBot {
    toggle: KeyToggle,
    state: TriggerState,
    was_enabled: bool,
}

impl TriggerBot {
    pub fn new() -> Self {
        Self {
            toggle: KeyToggle::new(),
            state: TriggerState::Idle,
            was_enabled: false,
        }
    }

    fn button(down: bool) -> MouseState {
        let mut state = MouseState::default();
        state.buttons[0] = Some(down);
        state
    }

    fn click_release(ctx: &UpdateContext) -> anyhow::Result<()> {
        ctx.cs2.send_mouse_state(&[Self::button(false)])?;
        Ok(())
    }

    /// Press and release in one input so the button cannot stay down between shots.
    fn click(ctx: &UpdateContext) -> anyhow::Result<()> {
        ctx.cs2.send_mouse_state(&[
            Self::button(false),
            Self::button(true),
            Self::button(false),
        ])?;
        Ok(())
    }

    fn should_be_active(&self, ctx: &UpdateContext) -> anyhow::Result<bool> {
        let settings = ctx.states.resolve::<AppSettings>(())?;
        let crosshair = ctx.states.resolve::<StateLocalCrosshair>(())?;

        let target = match crosshair.current_target() {
            Some(target) => target,
            None => return Ok(false),
        };

        if !target
            .entity_type
            .as_ref()
            .map(|t| t == "C_CSPlayerPawn")
            .unwrap_or(false)
        {
            return Ok(false);
        }

        let pawn_info = match ctx.states.resolve::<StatePawnInfo>(
            EntityHandle::<dyn C_CSPlayerPawn>::from_index(target.entity_id),
        ) {
            Ok(info) => info,
            Err(_) => return Ok(false),
        };
        if !settings.player_aim_enabled(pawn_info.player_name.as_deref()) {
            return Ok(false);
        }

        if settings.trigger_bot_team_check {
            let memory = ctx.states.resolve::<StateCS2Memory>(())?;
            let local_player_controller = ctx.states.resolve::<StateLocalPlayerController>(())?;
            let Some(local_player_controller) = local_player_controller
                .instance
                .value_reference(memory.view_arc())
            else {
                return Ok(false);
            };

            if pawn_info.team_id == local_player_controller.m_iTeamNum()? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl Enhancement for TriggerBot {
    fn update_settings(
        &mut self,
        ui: &imgui::Ui,
        settings: &mut AppSettings,
    ) -> anyhow::Result<bool> {
        let mut changed = false;
        for (index, key) in crate::settings::TRIGGER_SHOT_PRESET_KEYS.iter().enumerate() {
            if ui.is_key_pressed_no_repeat(*key) && settings.select_trigger_shot_preset(index) {
                log::debug!(
                    "Selected shoot duration numpad {} ({}ms)",
                    index + 1,
                    settings.trigger_bot_shot_duration
                );
                changed = true;
            }
        }
        Ok(changed)
    }

    fn update(&mut self, ctx: &UpdateContext) -> anyhow::Result<()> {
        let settings = ctx.states.resolve::<AppSettings>(())?;
        if self.toggle.update(
            &settings.trigger_bot_mode,
            ctx.input,
            &settings.key_trigger_bot,
        ) {
            ctx.cs2.add_metrics_record(
                obfstr!("feature-trigger-bot-toggle"),
                &format!(
                    "enabled: {}, mode: {:?}",
                    self.toggle.enabled, settings.trigger_bot_mode
                ),
            );
        }

        if !self.toggle.enabled {
            if self.was_enabled {
                let _ = Self::click_release(ctx);
                self.state = TriggerState::Idle;
            }
            self.was_enabled = false;
            return Ok(());
        }
        self.was_enabled = true;

        let should_shoot = self.should_be_active(ctx)?;

        match self.state {
            TriggerState::Idle => {
                if should_shoot {
                    let delay_min = settings
                        .trigger_bot_delay_min
                        .min(settings.trigger_bot_delay_max);
                    let delay_max = settings
                        .trigger_bot_delay_min
                        .max(settings.trigger_bot_delay_max);
                    let selected_delay = if delay_max == delay_min {
                        delay_min
                    } else {
                        let dist = Uniform::new_inclusive(delay_min, delay_max);
                        dist.sample(&mut rand::thread_rng())
                    };
                    self.state = TriggerState::Pending {
                        delay: selected_delay,
                        timestamp: Instant::now(),
                    };
                }
            }
            TriggerState::Pending { delay, timestamp } => {
                if timestamp.elapsed().as_millis() >= delay as u128 {
                    if settings.trigger_bot_check_target_after_delay && !should_shoot {
                        self.state = TriggerState::Idle;
                    } else {
                        Self::click(ctx)?;
                        self.state = TriggerState::Cooldown {
                            until: Instant::now() + Duration::from_millis(settings.trigger_bot_shot_duration as u64),
                        };
                    }
                }
            }
            TriggerState::Cooldown { until } => {
                if !should_shoot || Instant::now() >= until {
                    self.state = TriggerState::Idle;
                }
            }
        }

        Ok(())
    }

    fn render(
        &self,
        _states: &StateRegistry,
        _ui: &imgui::Ui,
        _unicode_text: &UnicodeTextRenderer,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
