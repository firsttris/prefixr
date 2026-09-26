use super::render_vkbasalt_conf;
use crate::models::VkBasaltSettings;

#[test]
fn renders_enabled_vkbasalt_effects_in_order() {
    let settings = VkBasaltSettings {
        enabled: true,
        sharpen: true,
        sharpness: 0.65,
        smaa: true,
    };

    assert_eq!(
        render_vkbasalt_conf(&settings),
        "effects = cas:smaa\ncasSharpness = 0.65\nsmaaEdgeDetection = luma\nsmaaThreshold = 0.05\nsmaaMaxSearchSteps = 32\nsmaaMaxSearchStepsDiag = 16\nsmaaCornerRounding = 25\nenableOnLaunch = True\n"
    );
}

#[test]
fn omits_disabled_vkbasalt_effects() {
    let settings = VkBasaltSettings {
        enabled: false,
        sharpen: false,
        sharpness: 0.4,
        smaa: false,
    };

    assert_eq!(
        render_vkbasalt_conf(&settings),
        "effects = \nenableOnLaunch = True\n"
    );
}
