use super::*;

#[test]
fn collects_options_and_groups_aliases() {
    let script = r#"
    self.check_environment("PROTON_ENABLE_HDR", "hdr")
    self.check_environment("PROTON_USE_HDR", "hdr")
    if not self.check_environment("PROTON_USE_WINED3D", "wined3d"):
        self.check_environment("PROTON_USE_WINED3D11", "wined3d")
    g_session.check_environment("PROTON_FSR4_UPGRADE", "fsr4")
    g_session.check_environment("PROTON_FSR4_UPGRADE", "fsr4")
    self.check_environment("ENABLE_VKBASALT", "vkbasalt")
    def check_environment(self, env_name, config_name):
    "#;
    let options = parse_proton_options(script);
    assert_eq!(
        options,
        vec![
            ProtonOption {
                env: "PROTON_ENABLE_HDR".into(),
                config: "hdr".into(),
                aliases: vec!["PROTON_USE_HDR".into()],
            },
            ProtonOption {
                env: "PROTON_USE_WINED3D".into(),
                config: "wined3d".into(),
                aliases: vec!["PROTON_USE_WINED3D11".into()],
            },
            ProtonOption {
                env: "PROTON_FSR4_UPGRADE".into(),
                config: "fsr4".into(),
                aliases: vec![],
            },
        ]
    );
}

#[test]
fn skips_calls_without_string_literals() {
    let script = r#"self.check_environment(name, "x") self.check_environment("A", cfg)"#;
    assert!(parse_proton_options(script).is_empty());
}
