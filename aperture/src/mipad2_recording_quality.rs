// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit quality candidate, not a claim about active MIUI runtime settings.
//! Vendor XML declares 15Mbps/AAC but carries a dummy MRD7/T100 header.
use gst::prelude::*;
use gst_pbutils::encoding_profile::EncodingProfileBuilder;
use std::sync::{LazyLock, Mutex};

static AAC_ORIGINAL_RANK: LazyLock<Mutex<Option<gst::Rank>>> = LazyLock::new(|| Mutex::new(None));

fn selected_aac_rank(enabled: bool, original: gst::Rank) -> gst::Rank {
    if enabled {
        original.max(gst::Rank::PRIMARY)
    } else {
        original
    }
}

pub(crate) fn configure_aac_rank(enabled: bool) {
    let mut original = AAC_ORIGINAL_RANK.lock().unwrap();
    // Never touch rank in a process which has not enabled this candidate.
    if !enabled && original.is_none() {
        return;
    }
    if let Some(factory) = gst::ElementFactory::find("avenc_aac") {
        let prior = *original.get_or_insert(factory.rank());
        factory.set_rank(selected_aac_rank(enabled, prior));
    }
}

pub(crate) fn policy(
    device: bool,
    h264: bool,
    hw: bool,
    android: bool,
    requested: bool,
    aac: bool,
) -> bool {
    device && h264 && hw && android && requested && aac
}

/// Diagnostic ablations never alter the normal joint-quality policy.
pub(crate) fn diagnostic_axes(enabled: bool, diagnostic: bool, axis: Option<&str>) -> (bool, bool) {
    if !enabled {
        return (false, false);
    }
    if !diagnostic {
        return (true, true);
    }
    match axis {
        Some("legacy") => (false, false),
        Some("video-only") => (true, false),
        Some("audio-only") => (false, true),
        _ => (true, true), // Invalid/missing input must not silently lower quality.
    }
}

pub(crate) fn video_kbps(enabled: bool, legacy: u32) -> u32 {
    if enabled { 15_000 } else { legacy }
}

pub(crate) fn aac_profile() -> gst_pbutils::EncodingAudioProfile {
    let format = gst::Caps::builder("audio/mpeg")
        .field("mpegversion", 4i32)
        .field("base-profile", "lc")
        .build();
    let raw = gst::Caps::builder("audio/x-raw")
        .field("rate", 48_000i32)
        .field("channels", 2i32)
        .build();
    // Pin the factory; encodebin also needs its rank raised in this process only.
    let properties = gst_pbutils::ElementProperties::builder_map()
        .item(
            gst_pbutils::ElementPropertiesMapItem::builder("avenc_aac")
                .field("bitrate", 192_000i32)
                .build(),
        )
        .build();
    gst_pbutils::EncodingAudioProfile::builder(&format)
        .restriction(&raw)
        .preset_name("avenc_aac")
        .element_properties(properties)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gst_pbutils::prelude::*;
    #[test]
    fn quality_policy_requires_every_capability_and_opt_in() {
        for mask in 0..64 {
            let b: Vec<_> = (0..6).map(|i| mask & (1 << i) != 0).collect();
            assert_eq!(policy(b[0], b[1], b[2], b[3], b[4], b[5]), mask == 63);
        }
    }
    #[test]
    fn quality_aac_rank_preserves_and_restores_original_policy() {
        for original in [
            gst::Rank::NONE,
            gst::Rank::MARGINAL,
            gst::Rank::PRIMARY,
            gst::Rank::PRIMARY + 100,
        ] {
            assert_eq!(selected_aac_rank(false, original), original);
            assert!(selected_aac_rank(true, original) >= gst::Rank::PRIMARY);
            assert!(selected_aac_rank(true, original) >= original);
        }
    }

    #[test]
    fn diagnostic_axes_are_explicitly_gated_and_preserve_joint_target() {
        for enabled in [false, true] {
            for diagnostic in [false, true] {
                for axis in [
                    None,
                    Some("joint"),
                    Some("legacy"),
                    Some("video-only"),
                    Some("audio-only"),
                    Some("typo"),
                ] {
                    let expected = if !enabled {
                        (false, false)
                    } else if !diagnostic {
                        (true, true)
                    } else {
                        match axis {
                            Some("legacy") => (false, false),
                            Some("video-only") => (true, false),
                            Some("audio-only") => (false, true),
                            _ => (true, true),
                        }
                    };
                    assert_eq!(diagnostic_axes(enabled, diagnostic, axis), expected);
                }
            }
        }
    }

    #[test]
    fn quality_bitrate_units_and_legacy_are_distinct() {
        assert_eq!(video_kbps(false, 2048), 2048);
        assert_eq!(video_kbps(true, 2048), 15_000);
        assert_eq!(video_kbps(true, 2048) * 1000, 15_000_000);
    }
    #[test]
    fn quality_audio_profile_is_explicit_aac_lc_not_any_mpeg() {
        gst::init().unwrap();
        let p = aac_profile();
        let caps = p.format();
        let f = caps.structure(0).unwrap();
        assert_eq!(f.name(), "audio/mpeg");
        assert_eq!(f.get::<i32>("mpegversion").unwrap(), 4);
        assert_eq!(f.get::<String>("base-profile").unwrap(), "lc");
        assert_eq!(p.preset_name().unwrap(), "avenc_aac");
        let raw = p.restriction().unwrap();
        let f = raw.structure(0).unwrap();
        assert_eq!(f.get::<i32>("rate").unwrap(), 48_000);
        assert_eq!(f.get::<i32>("channels").unwrap(), 2);
        assert!(
            p.element_properties()
                .unwrap()
                .to_string()
                .contains("192000")
        );
        let mp3 = gst::Caps::builder("audio/mpeg")
            .field("mpegversion", 1i32)
            .field("layer", 3i32)
            .build();
        assert!(caps.intersect(&mp3).is_empty());
    }
}
