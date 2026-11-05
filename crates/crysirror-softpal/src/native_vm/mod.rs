//! Main PAL engine crate.
//!
//! This crate is the integration layer above `pal-asset` and `pal-script`.
//! It owns the window/event loop boundary, the wgpu renderer, and the VM runtime.

pub mod abi;
pub mod animation;
#[cfg(feature = "desktop")]
pub mod app;
pub mod assets;
pub mod audio;
pub mod config;
pub mod debug;
pub mod effect;
pub mod engine;
pub mod event;
// Original C ABI source is retained, but the adapter uses the safe Engine port.
// No exported C host or platform shell is installed.
pub mod font;
pub mod fx;
pub mod image;
pub mod input;
pub mod list;
pub mod memory;
pub mod msprite;
pub mod platform_time;
#[cfg(feature = "desktop")]
pub mod renderer;
pub mod runtime;
mod save_format;
pub mod scene;
pub mod sprite;
pub mod system;
pub mod task;

#[cfg(all(feature = "desktop", target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm_entry;

pub use self::animation::{
    AnimationFrameRecord, AnimationHandle, PalAnimationFrameRecord, PalSequenceAnimationDesc,
    PalSheetAnimationDesc, SequenceAnimationDesc, SheetAnimationAxis, SheetAnimationDesc,
};
#[cfg(feature = "desktop")]
pub use self::app::{
    run_sena, run_sena_headless, DiagnosticAutoAdvance, DiagnosticClick,
    DiagnosticClickWhenHitEnabled, DiagnosticKeyEvent, DiagnosticPngAt, DiagnosticWheelEvent,
    SenaConfig,
};
pub use self::assets::{CoreAssets, FileDatLayout, GraphicIndex};
pub use self::audio::{
    AudioConfig, AudioHandle, AudioSystem, PalSoundGroup, PalSoundStatus, PalVolume,
};
pub use self::config::{
    load_system_ini, parse_ini_nls, parse_system_ini, ConfigDat, EngineStartupConfig, IniFile,
    IniSection, IniValue, SystemDat, TextLang,
};
pub use self::debug::{
    collect_frame_dump, pal_debug_enabled, DrawCmdDumpEntry, FrameDebugDump, RenderDumpEntry,
    SpriteDumpEntry,
};
pub use self::effect::{PalEffectState, PalEffectSystem};
pub use self::engine::{Engine, EngineConfig, EngineFrame, FrameTiming, TraceConfig};
pub use self::event::{InputEvent, MouseButton, PalEvent};
pub use self::font::{
    PalBitmapFontMetricsFooter32, PalBitmapGlyphRecord32, PalDrawTextRect32, PalFontFallback,
    PalFontProfile, PalFontSystem,
};
pub use self::input::{PalInputState, PalKey, PalMouseButton};
pub use self::list::{PalListHandle, PalListSystem};
pub use self::memory::{PalMemoryHandle, PalMemorySystem};
pub use self::msprite::{
    MSpriteFrameUpdate, MSpriteHandle, MSpriteSystem, MoviePlayback, MSPRITE_STATE_FINISHED,
    MSPRITE_STATE_LOOP, MSPRITE_STATE_PLAYING, MSPRITE_STATE_STOPPED,
};
#[cfg(feature = "desktop")]
pub use self::renderer::{
    pal_device_rect_to_clip_quad, scaled_rect, sprite_transform_debug, RenderOutcome,
    RenderTargetMetrics, Renderer, RendererConfig, ShaderProgram, SpriteTransformDebug,
};
pub use self::runtime::{
    FrameEvent, RuntimeError, RuntimeStatus, RuntimeTick, ScriptRuntime, ScriptRuntimeConfig,
    WaitRequest,
};
pub use self::scene::{
    DrawCommand, FrameScene, RectF, SceneTexture, SceneTextureFormat, SceneTextureId, SolidQuad,
    SpriteDraw,
};
pub use self::sprite::{
    MSpriteDecoderHandle, PalAnimationAxis, PalAnimationFlags, PalColor, PalPoint2, PalPoint3,
    PalRect, PalRenderMode, PalSize, PalSprite, PalSpriteInfo, PalVec3, RenderNode, RenderNodeId,
    SpriteDesc, SpriteFxEffect, SpriteFxHandle, SpriteHandle, SpriteKind, SpriteOptionAux,
    SpriteSurface, SpriteSurfaceId, SpriteSystem, SpriteTransition, SpriteTransitionHandle,
};
pub use self::system::{
    PalGesture, PalRandomState, PalRectI, PalSystemPathKind, PalSystemPaths, PalSystemState,
    PalWindowRequest,
};
pub use self::task::{
    AnimationHandle as TaskAnimHandle, TaskDumpEntry, TaskHandle, TaskSystem, BLOCKING_FLAG,
    TASK_POOL_CAPACITY,
};

pub(crate) mod host_paths;
