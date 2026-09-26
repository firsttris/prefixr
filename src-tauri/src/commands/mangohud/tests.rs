use super::render_conf;
use crate::models::MangoHudLayout;

#[test]
fn renders_enabled_mangohud_fields_and_layout() {
    let layout = MangoHudLayout {
        position: "bottom-right".to_string(),
        theme_color: "00ff88".to_string(),
        background_alpha: 0.8,
        round_corners: true,
        show_ram: true,
        show_vram: true,
        show_gamemode: true,
        show_driver: true,
        horizontal: true,
        ..MangoHudLayout::default()
    };

    assert_eq!(
        render_conf(&layout),
        "fps\nframetime\ncpu_stats\ngpu_stats\nram\nvram\ncpu_temp\ngpu_temp\ngamemode\nvulkan_driver\nhorizontal\nposition=bottom-right\nbackground_alpha=0.8\ntext_color=00ff88\nround_corners=10\n"
    );
}

#[test]
fn leaves_disabled_mangohud_toggles_out_of_the_file() {
    let layout = MangoHudLayout {
        show_fps: false,
        show_frametime: false,
        show_cpu: false,
        show_gpu: false,
        show_temps: false,
        round_corners: false,
        ..MangoHudLayout::default()
    };

    assert_eq!(
        render_conf(&layout),
        "position=top-left\nbackground_alpha=0.4\ntext_color=ffffff\n"
    );
}
