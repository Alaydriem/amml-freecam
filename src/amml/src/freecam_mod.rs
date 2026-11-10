use std::ffi::c_void;
use std::sync::atomic::AtomicPtr;

use amml_game::{Actor, CameraWriter, EntryId, EntryNeed, EntryResolver, GameEntries, GameError, HookOrder};
use amml_sdk::amml_abi::AmmlLogLevel;
use amml_sdk::{AmmlMod, Api, HookHandle, ModDirs, SdkError};

use crate::detours::{
    camera_instruction, camera_presets, chunk_publisher, client_update, pre_render_update, setup_camera,
    view_render_object,
};
use crate::game::PacketHandler;
use crate::services::FreecamRuntime;
use crate::types::FreecamFeature;

// the AMML entry point: load notes the folders, enable resolves the entries, installs the hooks and starts freecam,
// disable returns the camera, removes the markers and the hooks and stops it
pub struct FreecamMod;

impl FreecamMod {
    // the entries freecam cannot run without besides the camera write, the actor extras and the delivery: the
    // client, the player's pose, level, dimension and block source, and the three render hooks
    pub const REQUIRED: [EntryId; 14] = [
        EntryId::CLIENT_INSTANCE_UPDATE,
        EntryId::CLIENT_INSTANCE_GET_LOCAL_PLAYER,
        EntryId::CLIENT_INSTANCE_IS_WORLD_ACTIVE,
        EntryId::CLIENT_INSTANCE_IS_SHOWING_MENU,
        EntryId::ACTOR_STATE_VECTOR_COMPONENT,
        EntryId::ACTOR_ACTOR_ROTATION_COMPONENT,
        EntryId::STATE_VECTOR_COMPONENT_POS,
        EntryId::ACTOR_ROTATION_COMPONENT_ROT,
        EntryId::ACTOR_LEVEL,
        EntryId::ACTOR_GET_DIMENSION,
        EntryId::DIMENSION_GET_DIMENSION_ID,
        EntryId::ACTOR_GET_DIMENSION_BLOCK_SOURCE,
        EntryId::LEVEL_RENDERER_PLAYER_SETUP_CAMERA,
        EntryId::LEVEL_RENDERER_PLAYER_CREATE_VIEW_RENDER_OBJECT,
    ];

    // every entry freecam asks for: the required ones, then each optional feature's (markers, camera watch, look-at).
    // The projection read needs only the camera anchor, which the camera write requires
    pub fn wanted() -> Vec<(EntryId, EntryNeed)> {
        let required = Self::REQUIRED
            .iter()
            .chain(CameraWriter::ENTRIES.iter())
            .chain(Actor::EXTRA_ENTRIES.iter())
            .chain(FreecamFeature::DELIVERY_ENTRIES.iter())
            .map(|id| (*id, EntryNeed::Required));
        let optional = FreecamFeature::OPTIONAL.iter().flat_map(|feature| {
            FreecamFeature::entries(feature).iter().map(|id| (*id, EntryNeed::Optional { feature }))
        });
        let mut wanted: Vec<(EntryId, EntryNeed)> = Vec::new();
        for (id, need) in required.chain(optional) {
            if !wanted.iter().any(|(seen, _)| *seen == id) {
                wanted.push((id, need));
            }
        }
        wanted
    }

    // the entries, or an error when a required entry is missing
    pub fn resolve<R: EntryResolver>(resolver: &R) -> Result<GameEntries, GameError> {
        GameEntries::resolve(resolver, &Self::wanted())
    }

    // one line per feature that is off, naming it and the entry that turned it off
    pub fn off_lines(entries: &GameEntries) -> Vec<String> {
        entries.off_features().into_iter().map(|(feature, cause)| format!("{feature} is off ({cause})")).collect()
    }

    #[allow(unsafe_code)]
    fn hook(
        api: &Api,
        runtime: &FreecamRuntime,
        target: u64,
        order: i32,
        detour: *const c_void,
        original: &'static AtomicPtr<c_void>,
    ) -> Result<HookHandle, SdkError> {
        // SAFETY: `target` is the derived start of the game function each caller pairs with the detour written for
        // its signature (this in rcx, then the C++ parameters as the game passes them); each original slot belongs
        // to one hook only
        let hook = unsafe { api.hook(target, detour, order, original) }?;
        if runtime.keep_hook(hook) {
            return Ok(hook);
        }
        let _ = api.unhook(hook);
        Err(SdkError::Mod(format!("more than {} hooks", FreecamRuntime::HOOK_CAPACITY)))
    }

    fn remove_hooks(api: &Api, runtime: &FreecamRuntime) {
        for hook in runtime.take_hooks() {
            if let Err(error) = api.unhook(hook) {
                api.log(&Self::id(), AmmlLogLevel::WARN, &format!("unhook failed: {error}"));
            }
        }
    }

    // the three render hooks write the camera, so they run first; the four packet hooks only capture or report
    fn install_hooks(api: &Api, runtime: &FreecamRuntime, entries: &GameEntries) -> Result<(), SdkError> {
        let get = |id: EntryId| entries.get(id).ok_or_else(|| SdkError::MissingEntry(id.as_str().to_string()));
        let update = get(EntryId::CLIENT_INSTANCE_UPDATE)?;
        let setup = get(EntryId::LEVEL_RENDERER_PLAYER_SETUP_CAMERA)?;
        let view = get(EntryId::LEVEL_RENDERER_PLAYER_CREATE_VIEW_RENDER_OBJECT)?;
        let level_camera = get(EntryId::LEVEL_RENDERER_CAMERA_PRE_RENDER_UPDATE)?;
        let [chunk, presets, instruction] = PacketHandler::new(entries)
            .and_then(|handler| handler.hook_targets())
            .ok_or_else(|| SdkError::Mod("the ClientNetworkHandler vtable holds no handle overload".to_string()))?;
        let (write, read) = (HookOrder::WRITE, HookOrder::READ_AFTER_WRITERS);
        let hooks: [(u64, i32, *const c_void, &'static AtomicPtr<c_void>); 7] = [
            (update, read, client_update::detour as *const c_void, client_update::GUARD.original()),
            (setup, write, setup_camera::detour as *const c_void, setup_camera::GUARD.original()),
            (view, write, view_render_object::detour as *const c_void, view_render_object::GUARD.original()),
            (level_camera, write, pre_render_update::detour as *const c_void, pre_render_update::GUARD.original()),
            (chunk, read, chunk_publisher::detour as *const c_void, chunk_publisher::GUARD.original()),
            (presets, read, camera_presets::detour as *const c_void, camera_presets::GUARD.original()),
            (instruction, read, camera_instruction::detour as *const c_void, camera_instruction::GUARD.original()),
        ];
        for (target, order, detour, original) in hooks {
            Self::hook(api, runtime, target, order, detour, original)?;
        }
        Ok(())
    }
}

impl AmmlMod for FreecamMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "freecam";

    // runs under the loader lock: only the folders are noted, nothing starts a thread
    fn load(api: &Api) -> Result<(), SdkError> {
        let dirs = ModDirs::new(api, &Self::id())?;
        FreecamRuntime::install(&dirs.config_dir);
        Ok(())
    }

    fn enable(api: &Api) -> Result<(), SdkError> {
        let runtime = FreecamRuntime::get().ok_or_else(|| SdkError::Mod("load did not run".to_string()))?;
        let entries = Self::resolve(api).map_err(|error| SdkError::Mod(error.to_string()))?;
        let mut off_lines = Self::off_lines(&entries);
        if let Err(error) = Self::install_hooks(api, runtime, &entries) {
            Self::remove_hooks(api, runtime);
            return Err(error);
        }
        match runtime.start(entries) {
            Ok(refused) => off_lines.extend(refused),
            Err(error) => {
                Self::remove_hooks(api, runtime);
                return Err(SdkError::Mod(error));
            }
        }
        for line in &off_lines {
            api.log(&Self::id(), AmmlLogLevel::WARN, line);
        }
        if off_lines.is_empty() {
            api.log(&Self::id(), AmmlLogLevel::INFO, "features off: none");
        }
        Ok(())
    }

    // the camera goes back to the player and the markers go before any hook, while the handler is still captured;
    // then the hooks go, newest first, so no new frame or packet enters freecam; then the capture and the packets are
    // dropped and the registration ends
    fn disable(api: &Api) {
        let Some(runtime) = FreecamRuntime::get() else { return };
        runtime.send_shutdown();
        runtime.latch().raise();
        Self::remove_hooks(api, runtime);
        runtime.stop();
    }
}
