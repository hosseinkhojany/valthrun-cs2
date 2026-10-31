use std::time::{Duration, Instant};
use std::ffi::c_void;
use cs2::{
    CEntityIdentityEx, 
    CS2Model, ClassNameCache, MouseState, PlayerPawnState,
    StateCS2Memory, StateEntityList, StateLocalPlayerController,
    StatePawnInfo, StatePawnModelInfo,
};
use overlay::UnicodeTextRenderer;
use nalgebra::Vector2;


use windows::{
    core::PCWSTR,
    Win32::Foundation::{CloseHandle, HANDLE},
    Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingW, FILE_MAP_READ, UnmapViewOfFile,
        MEMORY_MAPPED_VIEW_ADDRESS
    },
    Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState,
        VK_LBUTTON,
    },
};

use super::Enhancement;
use crate::{
    settings::{
        AimTarget,
        AppSettings,
    },
    view::ViewController,
};

#[repr(C)]
struct RecoilSharedMemory {
    recoil_offset_x: i32,
    recoil_offset_y: i32,
}

pub struct HumanAimbot {
    start_time: Instant,
    fire_was_down: bool,
    /// Aim moves pause across a click so the game sees the press and release.
    fire_quiet_until: Instant,

    shm_handle: HANDLE,
    shm_ptr: *const RecoilSharedMemory,
}

unsafe impl Send for HumanAimbot {}
unsafe impl Sync for HumanAimbot {}

impl HumanAimbot {
    pub fn new() -> Self {
        let mut aimbot = Self {
            start_time: Instant::now(),
            fire_was_down: false,
            fire_quiet_until: Instant::now(),
            shm_handle: HANDLE(0), // Null handle
            shm_ptr: std::ptr::null(),
        };

        aimbot.connect_shared_memory();
        aimbot
    }

    fn connect_shared_memory(&mut self) {
        unsafe {
            let shm_name_str = "Global\\GodRecoilOffsets";
            let mut shm_name_wide: Vec<u16> = shm_name_str.encode_utf16().collect();
            shm_name_wide.push(0);

            println!("[AIMBOT] Connecting to shared memory: {}", shm_name_str);

            let handle = OpenFileMappingW(
                FILE_MAP_READ.0, 
                false, 
                PCWSTR(shm_name_wide.as_ptr())
            );

            if let Ok(h) = handle {
                if !h.is_invalid() && h.0 != 0 {
                    self.shm_handle = h;
                    
                    let mapped_view = MapViewOfFile(
                        h, 
                        FILE_MAP_READ, 
                        0, 
                        0, 
                        std::mem::size_of::<RecoilSharedMemory>()
                    );

                    // .Value ile asıl pointer'a erişiyoruz
                    if !mapped_view.Value.is_null() {
                        self.shm_ptr = mapped_view.Value as *const RecoilSharedMemory;
                        println!("[AIMBOT] Connected to shared memory! Ptr: {:?}", self.shm_ptr);
                    } else {
                        eprintln!("[AIMBOT] MapViewOfFile failed.");
                        let _ = CloseHandle(h);
                        self.shm_handle = HANDLE(0);
                    }
                } else {
                    eprintln!("[AIMBOT] Can't open shared memory handle.");
                }
            } else {
                eprintln!("[AIMBOT] Can't open shared memory handle.");
            }
        }
    }

    fn calculate_curved_move(&self, diff_x: f32, diff_y: f32, speed: f32, intensity: f32) -> (i32, i32) {
        let time_factor = self.start_time.elapsed().as_secs_f32() * 3.0;
        let curve_direction = time_factor.sin();
        
        let offset_x = -diff_y * intensity * curve_direction;
        let offset_y = diff_x * intensity * curve_direction;
        
        let target_dx = diff_x + offset_x;
        let target_dy = diff_y + offset_y;

        let smooth_factor = if speed > 100.0 { 100.0 } else { speed };
        let divisor = 100.0 - smooth_factor;
        
        let final_divisor = divisor / 5.0; 
        
        let raw_move_x = target_dx / final_divisor;
        let raw_move_y = target_dy / final_divisor;

        let move_x = if raw_move_x.abs() < 1.0 && raw_move_x.abs() > 0.05 {
            if raw_move_x > 0.0 { 1 } else { -1 }
        } else {
            raw_move_x.round() as i32
        };

        let move_y = if raw_move_y.abs() < 1.0 && raw_move_y.abs() > 0.05 {
            if raw_move_y > 0.0 { 1 } else { -1 }
        } else {
            raw_move_y.round() as i32
        };

        (move_x, move_y)
    }

    /// True when the FOV circle reaches the aim bone or any part of the player box.
    fn inside_fov(
        center: Vector2<f32>,
        radius: f32,
        bone: Vector2<f32>,
        bounds_min: Vector2<f32>,
        bounds_max: Vector2<f32>,
    ) -> bool {
        if (bone - center).norm() <= radius {
            return true;
        }

        let closest = Vector2::new(
            center.x.clamp(bounds_min.x, bounds_max.x),
            center.y.clamp(bounds_min.y, bounds_max.y),
        );
        (closest - center).norm() <= radius
    }
}

struct AimScreenTarget {
    bone: mint::Vector2<f32>,
    bounds_min: Vector2<f32>,
    bounds_max: Vector2<f32>,
}

impl HumanAimbot {
    /// Screen position of the selected aim bone for every enemy the aimbot can lock.
    fn collect_aim_points(
        &self,
        states: &utils_state::StateRegistry,
        settings: &AppSettings,
        target: AimTarget,
    ) -> anyhow::Result<Vec<AimScreenTarget>> {
        let memory = states.resolve::<StateCS2Memory>(())?;
        let view = states.resolve::<ViewController>(())?;
        let entities = states.resolve::<StateEntityList>(())?;
        let class_name_cache = states.resolve::<ClassNameCache>(())?;
        let local_controller = states.resolve::<StateLocalPlayerController>(())?;

        let Some(local_controller_ref) = local_controller.instance.value_reference(memory.view_arc()) else {
            return Ok(Vec::new());
        };

        let local_team_id = local_controller_ref.m_iPendingTeamNum()?;

        let mut points = Vec::new();
        for entity_identity in entities.entities() {
            let entity_class = class_name_cache.lookup(&entity_identity.entity_class_info()?)?;
            if !entity_class.map(|name| *name == "C_CSPlayerPawn").unwrap_or(false) {
                continue;
            }

            let pawn_state = states.resolve::<PlayerPawnState>(entity_identity.handle()?)?;
            if *pawn_state != PlayerPawnState::Alive {
                continue;
            }

            let pawn_info = states.resolve::<StatePawnInfo>(entity_identity.handle()?)?;
            if !settings.player_aim_enabled(pawn_info.player_name.as_deref()) {
                continue;
            }
            if pawn_info.team_id == local_team_id || pawn_info.player_health <= 0 {
                continue;
            }

            let pawn_model = states.resolve::<StatePawnModelInfo>(entity_identity.handle()?)?;
            let entry_model = states.resolve::<CS2Model>(pawn_model.model_address)?;
            let hull_height = entry_model.vhull_max.z - entry_model.vhull_min.z;
            let fallback_position = pawn_info.position
                + nalgebra::Vector3::new(
                    0.0,
                    0.0,
                    entry_model.vhull_min.z + hull_height * target.body_height_factor(),
                );

            let bone_position = target.bone_names().iter().find_map(|bone_name| {
                let bone_index = entry_model
                    .bones
                    .iter()
                    .position(|bone| bone.name == *bone_name)?;
                let bone_position = pawn_model.bone_states.get(bone_index)?.position;
                let distance = (bone_position - pawn_info.position).norm();
                (8.0..=90.0).contains(&distance).then_some(bone_position)
            });
            let aim_position = bone_position.unwrap_or(fallback_position);

            if let Some(screen_pos) = view.world_to_screen(&aim_position, true) {
                let bone = Vector2::new(screen_pos.x, screen_pos.y);
                let (mut bounds_min, mut bounds_max) = view
                    .calculate_box_2d(
                        &(entry_model.vhull_min + pawn_info.position),
                        &(entry_model.vhull_max + pawn_info.position),
                    )
                    .unwrap_or((bone, bone));
                bounds_min.x = bounds_min.x.min(bone.x);
                bounds_min.y = bounds_min.y.min(bone.y);
                bounds_max.x = bounds_max.x.max(bone.x);
                bounds_max.y = bounds_max.y.max(bone.y);
                points.push(AimScreenTarget {
                    bone: screen_pos,
                    bounds_min,
                    bounds_max,
                });
            }
        }

        Ok(points)
    }

    fn draw_aim_pointer(draw_list: &imgui::DrawListMut, pos: mint::Vector2<f32>) {
        let outline = [0.0, 0.0, 0.0, 0.85];
        let color = [1.0, 0.2, 0.2, 1.0];
        let arm = 8.0;

        draw_list
            .add_line([pos.x - arm, pos.y], [pos.x + arm, pos.y], outline)
            .thickness(3.0)
            .build();
        draw_list
            .add_line([pos.x, pos.y - arm], [pos.x, pos.y + arm], outline)
            .thickness(3.0)
            .build();
        draw_list
            .add_line([pos.x - arm, pos.y], [pos.x + arm, pos.y], color)
            .thickness(1.5)
            .build();
        draw_list
            .add_line([pos.x, pos.y - arm], [pos.x, pos.y + arm], color)
            .thickness(1.5)
            .build();
        draw_list
            .add_circle([pos.x, pos.y], 2.0, color)
            .filled(true)
            .build();
    }
}

impl Drop for HumanAimbot {
    fn drop(&mut self) {
        unsafe {
            if !self.shm_ptr.is_null() {

                let address_struct = MEMORY_MAPPED_VIEW_ADDRESS { 
                    Value: self.shm_ptr as *mut c_void 
                };
                let _ = UnmapViewOfFile(address_struct);
            }
            if !self.shm_handle.is_invalid() && self.shm_handle.0 != 0 {
                let _ = CloseHandle(self.shm_handle);
            }
        }
    }
}

impl Enhancement for HumanAimbot {
    fn update(&mut self, ctx: &crate::UpdateContext) -> anyhow::Result<()> {
        let settings = ctx.states.resolve::<AppSettings>(())?;

        if !settings.aim_bot_enabled {
            return Ok(());
        }

        let is_key_down = if let Some(key) = &settings.aim_bot_key {
            ctx.input.is_key_down(key.0)
        } else {
            false
        };

        if !is_key_down {
            self.start_time = Instant::now();
            return Ok(());
        }

        // Try again if not connected godrecoil
        //if self.shm_ptr.is_null() {
        //     self.connect_shared_memory();
        //}

        let view = ctx.states.resolve::<ViewController>(())?;
        let screen_center = Vector2::new(view.screen_bounds.x / 2.0, view.screen_bounds.y / 2.0);

        let mut best_target_pos: Option<Vector2<f32>> = None;
        let mut min_dist = f32::MAX;

        for target in self.collect_aim_points(ctx.states, &settings, settings.aim_bot_target)? {
            let bone = Vector2::new(target.bone.x, target.bone.y);
            if !Self::inside_fov(
                screen_center,
                settings.aim_bot_fov,
                bone,
                target.bounds_min,
                target.bounds_max,
            ) {
                continue;
            }

            let dist = (bone - screen_center).norm();
            if dist < min_dist {
                min_dist = dist;
                best_target_pos = Some(bone);
            }
        }

        if let Some(target_pos) = best_target_pos {
            let mut aim_x = target_pos.x;
            let mut aim_y = target_pos.y;

            // Recoil shifts where we pull the crosshair, not whether the target is inside the FOV.
            if !self.shm_ptr.is_null() {
                unsafe {
                    let recoil_data = &*self.shm_ptr;
                    aim_y += (recoil_data.recoil_offset_y as f32) * 0.75;
                    aim_x += (recoil_data.recoil_offset_x as f32) * 0.75;
                }
            }

            let diff_x = aim_x - screen_center.x;
            let diff_y = aim_y - screen_center.y;

            // Already on the bone. Extra 1px moves every frame block later clicks.
            if diff_x.hypot(diff_y) <= 2.5 {
                return Ok(());
            }

            let fire_down = unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16) & 0x8000 != 0 };
            if fire_down != self.fire_was_down {
                self.fire_was_down = fire_down;
                self.fire_quiet_until = Instant::now() + Duration::from_millis(70);
            }
            if Instant::now() < self.fire_quiet_until {
                return Ok(());
            }

            let (move_x, move_y) = self.calculate_curved_move(
                diff_x, 
                diff_y, 
                settings.aim_bot_smooth, 
                settings.aim_bot_curve
            );

            if move_x != 0 || move_y != 0 {
                ctx.cs2.send_mouse_state(&[MouseState {
                    last_x: move_x,
                    last_y: move_y, 
                    ..Default::default()
                }])?;
            }
        }

        Ok(())
    }

    fn render(
        &self,
        states: &utils_state::StateRegistry,
        ui: &imgui::Ui,
        _unicode_text: &UnicodeTextRenderer,
    ) -> anyhow::Result<()> {
        let settings = states.resolve::<AppSettings>(())?;
        if !settings.aim_bot_enabled {
            return Ok(());
        }

        let draw_list = ui.get_background_draw_list();
        let display_size = ui.io().display_size;

        if settings.aim_bot_draw_pointer {
            for target in self.collect_aim_points(states, &settings, settings.aim_bot_target)? {
                let pos = target.bone;
                if pos.x < 0.0 || pos.y < 0.0 || pos.x > display_size[0] || pos.y > display_size[1] {
                    continue;
                }
                Self::draw_aim_pointer(&draw_list, pos);
            }
        }

        if settings.aim_bot_draw_fov {
            let center = [display_size[0] / 2.0, display_size[1] / 2.0];
            draw_list
                .add_circle(center, settings.aim_bot_fov, [1.0, 1.0, 1.0, 1.0])
                .build();
        }

        Ok(())
    }
}