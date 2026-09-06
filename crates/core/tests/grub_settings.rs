#![deny(warnings)]

use bootcontrol_core::error::BootControlError;
use bootcontrol_core::grub_settings::{
    parse_grub_settings, validate_grub_settings, GrubSettings, GrubTimeoutStyle,
};

#[test]
fn parses_typed_values_and_documented_defaults() {
    let explicit = parse_grub_settings(
        "# user comment\nGRUB_TIMEOUT=12\nGRUB_TIMEOUT_STYLE=countdown\n\
         GRUB_DISABLE_OS_PROBER=true\nGRUB_DISABLE_RECOVERY=false\n",
    )
    .expect("valid settings");
    assert_eq!(explicit.timeout_seconds, 12);
    assert_eq!(explicit.timeout_style, GrubTimeoutStyle::Countdown);
    assert!(!explicit.detect_other_os);
    assert!(explicit.generate_recovery_entries);

    let defaults =
        parse_grub_settings("# no explicit supported settings\n").expect("defaults are valid");
    assert_eq!(defaults.timeout_seconds, 5);
    assert_eq!(defaults.timeout_style, GrubTimeoutStyle::Menu);
    assert!(defaults.detect_other_os);
    assert!(defaults.generate_recovery_entries);
}

#[test]
fn rejects_invalid_timeout_style_boolean_and_complex_bash() {
    for input in [
        "GRUB_TIMEOUT=-1\n",
        "GRUB_TIMEOUT=1000001\n",
        "GRUB_TIMEOUT_STYLE=fade\n",
        "GRUB_DISABLE_OS_PROBER=yes\n",
    ] {
        assert!(matches!(
            parse_grub_settings(input),
            Err(BootControlError::MalformedValue { .. })
        ));
    }
    assert!(matches!(
        parse_grub_settings("GRUB_TIMEOUT=$(id)\n"),
        Err(BootControlError::ComplexBashDetected { .. })
    ));
}

#[test]
fn validates_programmatically_constructed_settings() {
    let invalid = GrubSettings {
        timeout_seconds: 1_000_001,
        timeout_style: GrubTimeoutStyle::Menu,
        detect_other_os: true,
        generate_recovery_entries: true,
    };

    assert!(matches!(
        validate_grub_settings(&invalid),
        Err(BootControlError::MalformedValue { .. })
    ));
}
