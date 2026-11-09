//! The settings the launcher can switch. Every entry is a key the Linux client really reads; the defaults are
//! the client's built-in ones (what applies when the key is missing from the file).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Store {
    /// `WarcraftXL.cfg` in the client folder.
    Cfg,
    /// A game variable in `WTF/Config.wtf`.
    Wtf,
}

pub struct Setting {
    pub key: &'static str,
    pub store: Store,
    pub category: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub default: bool,
}

const fn s(key: &'static str, store: Store, category: &'static str, label: &'static str, hint: &'static str, default: bool) -> Setting {
    Setting { key, store, category, label, hint, default }
}

use Store::{Cfg, Wtf};

pub const SETTINGS: &[Setting] = &[
    // ---- Visuals ----
    s("WXL_VISUAL_ENVIRONMENT", Cfg, "VISUALS", "Modern visual environment", "Master switch for the modern water, atmosphere and lighting renderer", true),
    s("WXL_MODERN_WATER", Cfg, "VISUALS", "Modern water", "Reflective, animated water", true),
    s("WXL_WATER_RIPPLES", Cfg, "VISUALS", "Water ripples", "Ripples where things touch the water", true),
    s("WXL_VISUAL_MULTISAMPLING", Cfg, "VISUALS", "Multisampling for modern visuals", "Smoother edges on the modern effects; costs some performance", true),
    s("WXL_DISTANCE_FOG", Cfg, "VISUALS", "Distance fog", "Atmospheric fog toward the horizon", true),
    s("WXL_DISTANCE_HAZE", Cfg, "VISUALS", "Distance haze", "A soft haze on far terrain", true),
    s("WXL_FORWARD_SCATTERING", Cfg, "VISUALS", "Light scattering", "Glow around the sun and bright lights in fog", true),
    s("WXL_GOD_RAYS", Cfg, "VISUALS", "God rays", "Light shafts through the atmosphere", false),
    s("WXL_LOCAL_LIGHTS", Cfg, "VISUALS", "Local lights", "Nearby light sources light their surroundings", false),
    s("WXL_SURFACE_LIGHTS", Cfg, "VISUALS", "Torch and lamp lighting", "Torches and lamps light the terrain, models and buildings", true),
    s("WXL_GROUND_MIST", Cfg, "VISUALS", "Ground mist", "Low mist near the ground", false),
    s("WXL_TRANSPARENT_FOG", Cfg, "VISUALS", "Transparent fog", "Lets distant objects show through fog", false),
    s("WXL_GLOW_COMPENSATION", Cfg, "VISUALS", "Glow compensation", "Keeps bright effects from washing out in fog", true),
    s("WXL_INTERIOR_TRANSITIONS", Cfg, "VISUALS", "Smooth indoor and outdoor changes", "Fades the atmosphere when you walk through a door", true),
    s("WXL_EXTENDED_VIEW_DISTANCE", Cfg, "VISUALS", "Extended view distance", "See further than the original limit", true),
    s("WXL_DENSITY_VARIATION", Cfg, "VISUALS", "Fog density variation", "Fog thickness varies across the landscape", true),
    s("WXL_GRASS_MOTION", Cfg, "VISUALS", "Grass sway", "Grass moves in the wind and parts around you", true),
    s("WXL_LOOT_BEAM", Cfg, "VISUALS", "Loot beams", "Beams of light over corpses with loot", true),
    // ---- Models and gameplay ----
    s("WXL_MODERN_ASSETS", Cfg, "MODELS AND GAMEPLAY", "Modern models and textures", "Converts modern models and textures at load. Keep on with the supplied patches", true),
    s("WXL_COLLECTION_ARMOR", Cfg, "MODELS AND GAMEPLAY", "Collection armor models", "The server's extra armor models", false),
    s("WXL_UNIT_OVERLAYS", Cfg, "MODELS AND GAMEPLAY", "Target outline", "Outline around your target and other unit overlays", false),
    s("WXL_QUEST_RENDERING", Cfg, "MODELS AND GAMEPLAY", "Quest markers in the world", "Quest markers and objective rings drawn in the world", false),
    s("WXL_MUTE_CAST_FAILURE_SOUNDS", Cfg, "MODELS AND GAMEPLAY", "Mute cast-failure sounds", "Silences the error sound when a spell cannot be cast", false),
    s("WXL_GAMEOBJECT_EDITOR", Cfg, "MODELS AND GAMEPLAY", "Game-object editor (GM)", "The in-world editor overlay for game masters", true),
    // ---- Performance ----
    s("WXL_PARTICLE_THREADS", Cfg, "PERFORMANCE", "Threaded particle updates", "Update particle effects on worker threads", true),
    s("WXL_PARTICLE_BUILD_THREADS", Cfg, "PERFORMANCE", "Prebuilt particle geometry", "Build particle geometry ahead of drawing on worker threads", true),
    s("M2UseThreads", Wtf, "PERFORMANCE", "Multi-core model animation", "Animate models on several CPU threads (game setting)", true),
    s("weatherRainCap", Wtf, "PERFORMANCE", "Lighter rain", "Fewer rain drops and no distant splashes (game setting)", false),
    s("doodadFadeRebalance", Wtf, "PERFORMANCE", "Sooner fade of small objects", "Shrubs, small trees and flowers fade out nearer (game setting)", false),
    s("shadowCasterMinRadiusFilter", Wtf, "PERFORMANCE", "No shadows from small props", "Cups, bottles and small rocks stop casting shadows (game setting)", false),
    s("cullSubPixelDoodads", Wtf, "PERFORMANCE", "Skip tiny distant objects", "Objects under about 8 pixels are not drawn (game setting)", false),
    s("cullFogCutoff", Wtf, "PERFORMANCE", "Skip terrain past the fog", "May show a visible edge with some fog settings (game setting)", false),
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
            ("WXL_VISUAL_ENVIRONMENT", true), ("WXL_MODERN_WATER", true), ("WXL_WATER_RIPPLES", true), ("WXL_VISUAL_MULTISAMPLING", true),
            ("WXL_DISTANCE_FOG", true), ("WXL_DISTANCE_HAZE", false), ("WXL_FORWARD_SCATTERING", true), ("WXL_GOD_RAYS", true),
            ("WXL_LOCAL_LIGHTS", true), ("WXL_SURFACE_LIGHTS", true), ("WXL_GROUND_MIST", false), ("WXL_TRANSPARENT_FOG", true),
            ("WXL_GLOW_COMPENSATION", true), ("WXL_INTERIOR_TRANSITIONS", true), ("WXL_EXTENDED_VIEW_DISTANCE", true),
            ("WXL_DENSITY_VARIATION", false), ("WXL_GRASS_MOTION", true), ("WXL_LOOT_BEAM", true),
            ("WXL_PARTICLE_THREADS", true), ("WXL_PARTICLE_BUILD_THREADS", true), ("M2UseThreads", true),
            ("weatherRainCap", false), ("doodadFadeRebalance", false), ("shadowCasterMinRadiusFilter", false),
            ("cullSubPixelDoodads", false), ("cullFogCutoff", false),
        ],
    },
    Preset {
        name: "Quality",
        hint: "Everything visual on, including haze, density variation and ground mist",
        values: &[
            ("WXL_VISUAL_ENVIRONMENT", true), ("WXL_MODERN_WATER", true), ("WXL_WATER_RIPPLES", true), ("WXL_VISUAL_MULTISAMPLING", true),
            ("WXL_DISTANCE_FOG", true), ("WXL_DISTANCE_HAZE", true), ("WXL_FORWARD_SCATTERING", true), ("WXL_GOD_RAYS", true),
            ("WXL_LOCAL_LIGHTS", true), ("WXL_SURFACE_LIGHTS", true), ("WXL_GROUND_MIST", true), ("WXL_GLOW_COMPENSATION", true),
            ("WXL_INTERIOR_TRANSITIONS", true), ("WXL_EXTENDED_VIEW_DISTANCE", true), ("WXL_DENSITY_VARIATION", true),
            ("WXL_GRASS_MOTION", true), ("WXL_LOOT_BEAM", true),
            ("WXL_PARTICLE_THREADS", true), ("WXL_PARTICLE_BUILD_THREADS", true), ("M2UseThreads", true),
            ("weatherRainCap", false), ("doodadFadeRebalance", false), ("shadowCasterMinRadiusFilter", false), ("cullSubPixelDoodads", false),
        ],
    },
    Preset {
        name: "Performance",
        hint: "Heavier effects off, threading on, and the optional performance cuts on",
        values: &[
            ("WXL_VISUAL_MULTISAMPLING", false), ("WXL_DISTANCE_HAZE", false), ("WXL_GOD_RAYS", false), ("WXL_LOCAL_LIGHTS", false),
            ("WXL_SURFACE_LIGHTS", false), ("WXL_GROUND_MIST", false), ("WXL_WATER_RIPPLES", false),
            ("WXL_PARTICLE_THREADS", true), ("WXL_PARTICLE_BUILD_THREADS", true), ("M2UseThreads", true),
            ("weatherRainCap", true), ("doodadFadeRebalance", true), ("shadowCasterMinRadiusFilter", true), ("cullSubPixelDoodads", true),
        ],
    },
    Preset {
        name: "Safe mode",
        hint: "For troubleshooting: all threading and the newer effects off. Try this first if the game misbehaves",
        values: &[
            ("WXL_PARTICLE_THREADS", false), ("WXL_PARTICLE_BUILD_THREADS", false), ("M2UseThreads", false),
            ("WXL_SURFACE_LIGHTS", false), ("WXL_GRASS_MOTION", false), ("WXL_LOCAL_LIGHTS", false), ("WXL_GOD_RAYS", false),
            ("WXL_VISUAL_MULTISAMPLING", false),
            ("weatherRainCap", false), ("doodadFadeRebalance", false), ("shadowCasterMinRadiusFilter", false),
            ("cullSubPixelDoodads", false), ("cullFogCutoff", false),
        ],
    },
];

/// Same rule the game uses for `WxlProfileEnabled`: anything but a value starting with 0, n, N, f or F is on.
pub fn parse_bool(value: &str) -> bool {
    !matches!(value.chars().next(), Some('0' | 'n' | 'N' | 'f' | 'F'))
}

pub fn bool_text(on: bool) -> &'static str {
    if on { "1" } else { "0" }
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
