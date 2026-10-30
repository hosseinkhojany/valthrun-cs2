use std::{
    sync::{
        atomic::{
            AtomicBool,
            AtomicU32,
            Ordering,
        },
        Arc,
    },
    thread::JoinHandle,
    time::{
        Duration,
        Instant,
    },
};

use arc_swap::ArcSwap;
use cs2::{
    CEntityIdentityEx,
    CS2Handle,
    ClassNameCache,
    LocalCameraControllerTarget,
    PlayerPawnState,
    StateCS2Handle,
    StateCS2Memory,
    StateEntityList,
    StateLocalPlayerController,
    StatePawnInfo,
    StatePawnModelInfo,
    StateVariable,
};
use utils_state::StateRegistry;

/// A single player within an [`EspSnapshot`].
#[derive(Debug, Clone)]
pub struct SnapshotPlayer {
    pub info: StatePawnInfo,
    pub model_address: u64,
    pub bone_positions: Vec<nalgebra::Vector3<f32>>,
}

/// An immutable, plain-data view of everything the player ESP needs.
/// Produced by the background memory reader thread and consumed
/// lock-free by the render thread.
#[derive(Debug, Default)]
pub struct EspSnapshot {
    /// When the contained game state has been captured
    pub captured_at: Option<Instant>,

    pub local_team_id: u8,
    pub view_target_entity_id: Option<u32>,

    pub players: Vec<SnapshotPlayer>,
}

/// Statistics about the background memory reader thread.
#[derive(Debug, Default)]
pub struct EspReaderStats {
    /// Measured poll rate (in Hz) of the reader thread
    pub poll_rate: f32,
}

pub type SharedEspReaderStats = Arc<ArcSwap<EspReaderStats>>;

pub type StateEspReaderStats = StateVariable<SharedEspReaderStats>;

/// Background memory reader.
///
/// All driver memory reads required for the player ESP are performed on a
/// dedicated thread at a fixed rate. The render thread never touches the
/// driver and only consumes the latest published [`EspSnapshot`], hence
/// driver latency spikes can no longer stall a rendered frame.
pub struct EspReader {
    shared: Arc<ArcSwap<EspSnapshot>>,
    enabled: Arc<AtomicBool>,
    poll_interval_us: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,

    stats: SharedEspReaderStats,

    _thread: JoinHandle<()>,
}

impl EspReader {
    pub fn start(cs2: Arc<CS2Handle>, poll_rate_hz: f32) -> anyhow::Result<Self> {
        let shared = Arc::new(ArcSwap::from_pointee(EspSnapshot::default()));
        let enabled = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let poll_interval_us = Arc::new(AtomicU32::new(hz_to_interval_us(poll_rate_hz)));
        let stats: SharedEspReaderStats =
            Arc::new(ArcSwap::from_pointee(EspReaderStats::default()));

        let thread = {
            let shared = shared.clone();
            let enabled = enabled.clone();
            let shutdown = shutdown.clone();
            let poll_interval_us = poll_interval_us.clone();
            let stats = stats.clone();

            std::thread::Builder::new()
                .name("esp-memory-reader".to_string())
                .spawn(move || {
                    reader_main(cs2, shared, enabled, poll_interval_us, shutdown, stats)
                })?
        };

        Ok(Self {
            shared,
            enabled,
            poll_interval_us,
            shutdown,
            stats,
            _thread: thread,
        })
    }

    /// The latest published snapshot (lock-free).
    pub fn latest(&self) -> Arc<EspSnapshot> {
        self.shared.load_full()
    }

    /// Whether the reader should actively poll game memory.
    /// When disabled the reader idles at a low rate.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_poll_rate_hz(&self, poll_rate_hz: f32) {
        self.poll_interval_us
            .store(hz_to_interval_us(poll_rate_hz), Ordering::Relaxed);
    }

    pub fn stats(&self) -> SharedEspReaderStats {
        self.stats.clone()
    }
}

impl Drop for EspReader {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        /*
         * We intentionally do not join the thread here.
         * A read currently stuck in the driver would delay the
         * application shutdown. All reads are stateless, hence
         * letting the thread exit with the process is safe.
         */
    }
}

fn hz_to_interval_us(hz: f32) -> u32 {
    let hz = hz.clamp(1.0, 1000.0);
    (1_000_000.0 / hz) as u32
}

fn reader_main(
    cs2: Arc<CS2Handle>,
    shared: Arc<ArcSwap<EspSnapshot>>,
    enabled: Arc<AtomicBool>,
    poll_interval_us: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    stats: SharedEspReaderStats,
) {
    log::debug!("ESP memory reader thread started");

    let mut states = StateRegistry::new(1024 * 8);
    if let Err(error) = states
        .set(StateCS2Handle::new(cs2.clone()), ())
        .and_then(|_| states.set(StateCS2Memory::new(cs2.create_memory_view()), ()))
    {
        log::error!("ESP memory reader failed to initialize: {:#}", error);
        return;
    }

    /*
     * Double buffering:
     * We alternate between two snapshot buffers. While the render thread
     * reads the published buffer, we rewrite the other one. Once the
     * render thread released its reference, the buffer can be rewritten
     * in place - avoiding any allocations while polling.
     */
    let mut buffers = [
        Arc::new(EspSnapshot::default()),
        Arc::new(EspSnapshot::default()),
    ];
    let mut published_index = 0usize;

    let mut last_poll = Instant::now();
    let mut measured_poll_rate = 0.0f32;
    let mut stats_counter = 0u32;

    while !shutdown.load(Ordering::Relaxed) {
        let loop_start = Instant::now();

        if !enabled.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        states.invalidate_states();

        let write_index = 1 - published_index;
        let buffer = match Arc::get_mut(&mut buffers[write_index]) {
            Some(buffer) => buffer,
            None => {
                /* the render thread still holds this snapshot */
                buffers[write_index] = Arc::new(EspSnapshot::default());
                Arc::get_mut(&mut buffers[write_index]).expect("freshly allocated")
            }
        };

        match build_snapshot(&states, buffer) {
            Ok(()) => {
                buffer.captured_at = Some(loop_start);
                shared.store(buffers[write_index].clone());
                published_index = write_index;

                let elapsed = loop_start.duration_since(last_poll).as_secs_f32();
                last_poll = loop_start;
                if elapsed > 0.0 {
                    let instant_rate = 1.0 / elapsed;
                    measured_poll_rate = if measured_poll_rate <= 0.0 {
                        instant_rate
                    } else {
                        measured_poll_rate * 0.95 + instant_rate * 0.05
                    };
                }
            }
            Err(error) => {
                log::trace!("ESP memory reader update failed: {:#}", error);
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        }

        stats_counter += 1;
        if stats_counter >= 60 {
            stats_counter = 0;
            stats.store(Arc::new(EspReaderStats {
                poll_rate: measured_poll_rate,
            }));
        }

        let poll_interval = Duration::from_micros(poll_interval_us.load(Ordering::Relaxed) as u64);
        let work_time = loop_start.elapsed();
        if work_time < poll_interval {
            std::thread::sleep(poll_interval - work_time);
        }
    }

    log::debug!("ESP memory reader thread stopped");
}

fn build_snapshot(states: &StateRegistry, out: &mut EspSnapshot) -> anyhow::Result<()> {
    let entities = states.resolve::<StateEntityList>(())?;
    let class_name_cache = states.resolve::<ClassNameCache>(())?;
    let memory = states.resolve::<StateCS2Memory>(())?;

    let local_player_controller = states.resolve::<StateLocalPlayerController>(())?;
    let Some(local_player_controller) = local_player_controller
        .instance
        .value_reference(memory.view_arc())
    else {
        out.players.clear();
        out.view_target_entity_id = None;
        return Ok(());
    };

    out.local_team_id = local_player_controller.m_iPendingTeamNum()?;

    let view_target = states.resolve::<LocalCameraControllerTarget>(())?;
    out.view_target_entity_id = view_target.target_entity_id;
    let Some(view_target_entity_id) = view_target.target_entity_id else {
        out.players.clear();
        return Ok(());
    };

    out.players.clear();
    out.players.reserve(16);

    for entity_identity in entities.entities() {
        if entity_identity.handle::<()>()?.get_entity_index() == view_target_entity_id {
            continue;
        }

        let entity_class = class_name_cache.lookup(&entity_identity.entity_class_info()?)?;
        if !entity_class
            .map(|name| *name == "C_CSPlayerPawn")
            .unwrap_or(false)
        {
            /* entity is not a player pawn */
            continue;
        }

        let pawn_state = states.resolve::<PlayerPawnState>(entity_identity.handle()?)?;
        if *pawn_state != PlayerPawnState::Alive {
            continue;
        }

        let pawn_info = states.resolve::<StatePawnInfo>(entity_identity.handle()?)?;
        if pawn_info.player_health <= 0 || pawn_info.player_name.is_none() {
            continue;
        }

        let pawn_model = states.resolve::<StatePawnModelInfo>(entity_identity.handle()?)?;

        out.players.push(SnapshotPlayer {
            info: pawn_info.clone(),
            model_address: pawn_model.model_address,
            bone_positions: pawn_model
                .bone_states
                .iter()
                .map(|bone| bone.position)
                .collect(),
        });
    }

    Ok(())
}
