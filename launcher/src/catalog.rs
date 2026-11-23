//! The settings the launcher can switch. Every entry is a key the Linux client really reads; the defaults are
//! the client's built-in ones (what applies when the key is missing from the file).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Store {
    /// `WarcraftXL.cfg` in the client folder.
    Cfg,
    /// A game variable in `WTF/Config.wtf`.
    Wtf,
    /// An environment variable for the game, kept in `FTA-Linux/debug.env` and exported by `play.sh`.
    Env,
}

pub struct Setting {
    pub key: &'static str,
    pub store: Store,
    pub category: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub default: bool,
    /// What is written when the box is ticked / cleared (`1` and `0` for the ordinary on/off settings).
    pub on: &'static str,
    pub off: &'static str,
}

impl Setting {
    /// Whether `value`, as found in the file, counts as ticked.
    pub fn is_on(&self, value: &str) -> bool {
        match self.store {
            Store::Env => value.trim() != self.off,
            _ => parse_bool(value),
        }
    }

    pub fn text(&self, on: bool) -> &'static str {
        if on { self.on } else { self.off }
    }
}

const fn s(key: &'static str, store: Store, category: &'static str, label: &'static str, hint: &'static str, default: bool) -> Setting {
    Setting { key, store, category, label, hint, default, on: "1", off: "0" }
}

/// A debug switch (environment variable) that is off unless its value differs from `off`.
const fn dbg(key: &'static str, label: &'static str, hint: &'static str, on: &'static str, off: &'static str) -> Setting {
    Setting { key, store: Store::Env, category: DEBUG_CATEGORY, label, hint, default: false, on, off }
}

use Store::{Cfg, Wtf};

const DEBUG_CATEGORY: &str = "DEBUGGING (takes effect at the next game start; logs go to Logs/)";

pub const SETTINGS: &[Setting] = &[
    // ---- Visuals ----
    s("WXL_VISUAL_ENVIRONMENT", Cfg, "VISUALS", "Modern visual environment", "Master switch for the modern water, atmosphere and lighting renderer. Fog, water, shafts, mist, lamps and the rest are adjusted in game: Video options > Effects > Visuals & Guidance (tick Use these settings)", true),
    s("WXL_VISUAL_MULTISAMPLING", Cfg, "VISUALS", "Multisampling for modern visuals", "Smoother edges on the modern effects; costs some performance", true),
    s("WXL_DISTANCE_HAZE", Cfg, "VISUALS", "Distance haze", "A soft haze on far terrain", true),
    s("WXL_FORWARD_SCATTERING", Cfg, "VISUALS", "Light scattering", "Glow around the sun and bright lights in fog", true),
    s("WXL_SURFACE_LIGHTS", Cfg, "VISUALS", "Torch and lamp lighting", "Torches and lamps light the terrain, models and buildings", true),
    s("WXL_GLOW_COMPENSATION", Cfg, "VISUALS", "Glow compensation", "Keeps bright effects from washing out in fog", true),
    s("WXL_INTERIOR_TRANSITIONS", Cfg, "VISUALS", "Smooth indoor and outdoor changes", "Fades the atmosphere when you walk through a door", true),
    s("WXL_EXTENDED_VIEW_DISTANCE", Cfg, "VISUALS", "Extended view distance", "See further than the original limit", true),
    s("WXL_GRASS_MOTION", Cfg, "VISUALS", "Grass sway", "Grass moves in the wind and parts around you", true),
    s("WXL_LOOT_BEAM", Cfg, "VISUALS", "Loot beams", "Beams of light over corpses with loot", true),
    // ---- Models and gameplay ----
    s("WXL_MODERN_ASSETS", Cfg, "MODELS AND GAMEPLAY", "Modern models and textures", "Converts modern models and textures at load. Keep on with the supplied patches", true),
    s("WXL_COLLECTION_ARMOR", Cfg, "MODELS AND GAMEPLAY", "Collection armor models", "The server's extra armor models", false),
    s("WXL_UNIT_OVERLAYS", Cfg, "MODELS AND GAMEPLAY", "Target outline", "Outline around your target and other unit overlays", false),
    s("WXL_QUEST_RENDERING", Cfg, "MODELS AND GAMEPLAY", "Quest markers in the world", "Quest markers and objective rings drawn in the world", false),
    s("WXL_MUTE_CAST_FAILURE_SOUNDS", Cfg, "MODELS AND GAMEPLAY", "Mute cast-failure sounds", "Silences the error sound when a spell cannot be cast", false),
    s("WXL_MUTE_MURLOC_VOICE", Cfg, "MODELS AND GAMEPLAY", "Mute Murloc voices", "Silences the Murloc \"mrgl\" cries; a Murloc alt bot makes one on every stride", false),
    s("WXL_GAMEOBJECT_EDITOR", Cfg, "MODELS AND GAMEPLAY", "Game-object editor (GM)", "The in-world editor overlay for game masters", true),
    Setting { key: "WXL_PARTICLE_SPEED_FLIP", store: Store::Env, category: "MODELS AND GAMEPLAY", label: "Fix retail effect direction", hint: "Retail fire and smoke effects were upside down (torch flames pointing at the ground); this turns them the right way up. Switch off only to compare", default: true, on: "1", off: "0" },
    Setting { key: "WXL_WORLD_TEXTURE_LOOPS", store: Store::Env, category: "MODELS AND GAMEPLAY", label: "Animate retail waterfalls", hint: "Retail waterfalls and other scrolling-water models were frozen; this gives them a working animation clock. Switch off only to compare", default: true, on: "1", off: "0" },
    Setting { key: "WXL_WMO_MOCV_SCALE", store: Store::Env, category: "MODELS AND GAMEPLAY", label: "Fix retail building lighting", hint: "Retail houses were lit as if made of glowing paint, bright and shadowless at night; this lets the world light them instead. Switch off only to compare", default: true, on: "0.05", off: "1" },
    // ---- Performance ----
    s("WXL_PARTICLE_THREADS", Cfg, "PERFORMANCE", "Threaded particle updates", "Update particle effects on worker threads", true),
    s("WXL_PARTICLE_BUILD_THREADS", Cfg, "PERFORMANCE", "Prebuilt particle geometry", "Build particle geometry ahead of drawing on worker threads", true),
    s("M2UseThreads", Wtf, "PERFORMANCE", "Multi-core model animation", "Animate models on several CPU threads (game setting)", true),
    Setting { key: "WXL_COMPOSITOR_1024", store: Store::Env, category: "PERFORMANCE", label: "1024 character skins", hint: "Character skins are composed at 1024 pixels when the game's component texture level is 10. Switch off to fall back to 512 if faces or skins look wrong or video memory is tight", default: true, on: "1", off: "0" },
    Setting { key: "WXL_TERRAIN_AREA_REPLAY", store: Store::Env, category: "PERFORMANCE", label: "Faster terrain drawing", hint: "Reuses the terrain draw list while the view has not changed. Switch off only to compare or if terrain flickers", default: true, on: "1", off: "0" },
    Setting { key: "WXL_M2_KEYED_SORT", store: Store::Env, category: "PERFORMANCE", label: "Sorted model drawing", hint: "Draws opaque models in an order that saves state changes. Switch off only to compare or if models draw in the wrong order", default: true, on: "1", off: "0" },
    s("weatherRainCap", Wtf, "PERFORMANCE", "Lighter rain", "Fewer rain drops and no distant splashes (game setting)", false),
    s("doodadFadeRebalance", Wtf, "PERFORMANCE", "Sooner fade of small objects", "Shrubs, small trees and flowers fade out nearer (game setting)", false),
    s("shadowCasterMinRadiusFilter", Wtf, "PERFORMANCE", "No shadows from small props", "Cups, bottles and small rocks stop casting shadows (game setting)", false),
    s("cullSubPixelDoodads", Wtf, "PERFORMANCE", "Skip tiny distant objects", "Objects under about 8 pixels are not drawn (game setting)", false),
    s("cullFogCutoff", Wtf, "PERFORMANCE", "Skip terrain past the fog", "May show a visible edge with some fog settings (game setting)", false),
    // ---- Debugging: for finding problems; they write extra log lines or show errors, and cost some speed ----
    s("scriptErrors", Wtf, DEBUG_CATEGORY, "Show Lua errors", "Addon errors pop up in the game instead of staying silent (game setting)", false),
    s("scriptProfile", Wtf, DEBUG_CATEGORY, "Addon CPU profiling", "Lets addons measure their own CPU use; slows the UI a little (game setting)", false),
    dbg("WXL_NATIVE_DIAGNOSTICS", "Asset and equipment trace", "Logs model, texture and equipment path decisions to the launch log", "1", "0"),
    dbg("FTA_TRACE_JUMP", "Jump and root trace", "For \"cannot jump after a root or stun\": logs every refused jump with the movement flags that blocked it, and every root and unroot the game applies, to the launch log", "1", "0"),
    dbg("WXL_PROFILE_LOG", "Profiler summary every 5 seconds", "Logs frame time and timings per zone (water, particles, weather, ...) to the launch log", "5", "0"),
    dbg("WXL_PROFILE_GPU", "GPU time per pass", "Adds the GPU time of the main render passes to the profiler summary (needs the profiler summary above). Costs a little speed", "1", "0"),
    dbg("WXL_PROFILE_UI_DETAIL", "Interface time breakdown", "Splits the profiler's interface time by callback and addon update (needs the profiler summary above)", "1", "0"),
    dbg("WXL_PROFILE_M2_DRAWS", "Model draw breakdown", "Counts model draws by type and how many were merged (needs the profiler summary above)", "1", "0"),
    dbg("WXL_SOFT_PARTICLE_TIMING", "Soft particle timing", "Logs the GPU time of the soft particle depth step to Logs/VisualEnvironment.log (only when Soft particles are on in game)", "1", "0"),
    dbg("WXL_VERBOSE_ASSET_LOGS", "Per-asset transform log (developers)", "Logs how every modern model and texture is converted at load. Very long", "1", "0"),
    dbg("WXL_FOG_TRACE", "Fog and light pass trace", "Logs the state of the fog and light passes every 300 frames", "1", "0"),
    dbg("FTA_TRACE_PORTRAIT", "Portrait camera trace", "Logs how character portrait and preview cameras are set up", "1", "0"),
    dbg("FTA_TRACE_UNINVITE", "Party-error trace", "Logs where \"is not in your party\" errors come from", "1", "0"),
    dbg("WXL_SAMPLE_PROFILE", "Call-stack sampling (developers)", "500 samples a second; kill -USR1 on the game writes Logs/samples.txt. Can slow or hang the game", "500", "0"),
    dbg("WXL_HEAP_CHECK", "Heap checker (developers)", "Guards every allocation and logs overflows and use-after-free with the call site (WXL_HEAP_CHECK lines in the launch log). Slows the game and uses more memory", "1", "0"),
    dbg("WXL_SHADOW_MIN_INFLUENCES", "Old shadow shader choice (comparison)", "Brings back the stock shadow selection, so shields cast a wrong shadow again. Only to compare", "0", "1"),
];

pub struct Preset {
    pub name: &'static str,
    pub hint: &'static str,
    /// Only the listed settings change; everything else is left as it is.
    pub values: &'static [(&'static str, bool)],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Recommended",
        hint: "The settings From The Ashes ships with, threading on, the optional performance cuts off",
        values: &[
            ("WXL_VISUAL_ENVIRONMENT", true), ("WXL_VISUAL_MULTISAMPLING", true),
            ("WXL_DISTANCE_HAZE", false), ("WXL_FORWARD_SCATTERING", true), ("WXL_SURFACE_LIGHTS", true),
            ("WXL_GLOW_COMPENSATION", true), ("WXL_INTERIOR_TRANSITIONS", true), ("WXL_EXTENDED_VIEW_DISTANCE", true),
            ("WXL_GRASS_MOTION", true), ("WXL_LOOT_BEAM", true),
            ("WXL_PARTICLE_THREADS", true), ("WXL_PARTICLE_BUILD_THREADS", true), ("M2UseThreads", true),
            ("WXL_COMPOSITOR_1024", true), ("WXL_TERRAIN_AREA_REPLAY", true), ("WXL_M2_KEYED_SORT", true),
            ("weatherRainCap", false), ("doodadFadeRebalance", false), ("shadowCasterMinRadiusFilter", false),
            ("cullSubPixelDoodads", false), ("cullFogCutoff", false),
        ],
    },
    Preset {
        name: "Quality",
        hint: "Every visual setting the launcher controls on, including haze and light scattering (fog, mist, shafts and water are set in game)",
        values: &[
            ("WXL_VISUAL_ENVIRONMENT", true), ("WXL_VISUAL_MULTISAMPLING", true),
            ("WXL_DISTANCE_HAZE", true), ("WXL_FORWARD_SCATTERING", true), ("WXL_SURFACE_LIGHTS", true), ("WXL_GLOW_COMPENSATION", true),
            ("WXL_INTERIOR_TRANSITIONS", true), ("WXL_EXTENDED_VIEW_DISTANCE", true),
            ("WXL_GRASS_MOTION", true), ("WXL_LOOT_BEAM", true),
            ("WXL_PARTICLE_THREADS", true), ("WXL_PARTICLE_BUILD_THREADS", true), ("M2UseThreads", true),
            ("WXL_COMPOSITOR_1024", true), ("WXL_TERRAIN_AREA_REPLAY", true), ("WXL_M2_KEYED_SORT", true),
            ("weatherRainCap", false), ("doodadFadeRebalance", false), ("shadowCasterMinRadiusFilter", false), ("cullSubPixelDoodads", false),
        ],
    },
    Preset {
        name: "Performance",
        hint: "Heavier effects off, threading and the fast paths on, and the optional performance cuts on",
        values: &[
            ("WXL_VISUAL_MULTISAMPLING", false), ("WXL_DISTANCE_HAZE", false), ("WXL_SURFACE_LIGHTS", false),
            ("WXL_PARTICLE_THREADS", true), ("WXL_PARTICLE_BUILD_THREADS", true), ("M2UseThreads", true),
            ("WXL_TERRAIN_AREA_REPLAY", true), ("WXL_M2_KEYED_SORT", true),
            ("weatherRainCap", true), ("doodadFadeRebalance", true), ("shadowCasterMinRadiusFilter", true), ("cullSubPixelDoodads", true),
        ],
    },
    Preset {
        name: "Debug logging",
        hint: "Extra logging for a bug report: Lua errors shown, the asset trace and a profiler summary",
        values: &[("scriptErrors", true), ("WXL_NATIVE_DIAGNOSTICS", true), ("WXL_PROFILE_LOG", true)],
    },
    Preset {
        name: "Safe mode",
        hint: "For troubleshooting: all threading, the newer effects and the 1024 skin, terrain and sorting fast paths off. Try this first if the game misbehaves",
        values: &[
            ("WXL_PARTICLE_THREADS", false), ("WXL_PARTICLE_BUILD_THREADS", false), ("M2UseThreads", false),
            ("WXL_SURFACE_LIGHTS", false), ("WXL_GRASS_MOTION", false),
            ("WXL_VISUAL_MULTISAMPLING", false),
            ("WXL_COMPOSITOR_1024", false), ("WXL_TERRAIN_AREA_REPLAY", false), ("WXL_M2_KEYED_SORT", false),
            ("weatherRainCap", false), ("doodadFadeRebalance", false), ("shadowCasterMinRadiusFilter", false),
            ("cullSubPixelDoodads", false), ("cullFogCutoff", false),
        ],
    },
];

/// Same rule the game uses for `WxlProfileEnabled`: anything but a value starting with 0, n, N, f or F is on.
pub fn parse_bool(value: &str) -> bool {
    !matches!(value.chars().next(), Some('0' | 'n' | 'N' | 'f' | 'F'))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_key_exists_exactly_once() {
        for p in PRESETS {
            for (key, _) in p.values {
                assert_eq!(SETTINGS.iter().filter(|s| s.key == *key).count(), 1, "{} in {}", key, p.name);
            }
            let mut keys: Vec<_> = p.values.iter().map(|(k, _)| *k).collect();
            keys.sort();
            keys.dedup();
            assert_eq!(keys.len(), p.values.len(), "duplicate key in {}", p.name);
        }
    }

    #[test]
    fn setting_keys_are_unique() {
        let mut keys: Vec<_> = SETTINGS.iter().map(|s| s.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), SETTINGS.len());
    }

    #[test]
    fn bool_rule_matches_the_game() {
        for v in ["0", "no", "No", "false", "False", "f"] {
            assert!(!parse_bool(v), "{v}");
        }
        for v in ["1", "yes", "true", "on", "2"] {
            assert!(parse_bool(v), "{v}");
        }
    }
}
