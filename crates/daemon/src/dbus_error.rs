//! D-Bus error mapping for BootControl.
//!
//! # Problem z `fdo::Error::Failed`
//!
//! `zbus::fdo::Error::Failed` zawsze propaguje nazwę
//! `org.freedesktop.DBus.Error.Failed`. Klient nie może rozróżnić
//! `StateMismatch` od `PolkitDenied` bez parsowania wiadomości tekstowej —
//! co łamie kontrakt z `ARCHITECTURE.md`.
//!
//! # Rozwiązanie — `#[derive(DBusError)]`
//!
//! [`DaemonError`] implementuje `zbus::DBusError` via derive macro.
//! Każdy wariant jest propagowany po D-Bus z **własną strukturalną nazwą**
//! w przestrzeni `org.bootcontrol.Error.*`:
//!
//! | Wariant                 | Nazwa D-Bus                                         |
//! |-------------------------|-----------------------------------------------------|
//! | `StateMismatch`         | `org.bootcontrol.Error.StateMismatch`               |
//! | `KeyNotFound`           | `org.bootcontrol.Error.KeyNotFound`                 |
//! | `MalformedValue`        | `org.bootcontrol.Error.MalformedValue`              |
//! | `ComplexBashDetected`   | `org.bootcontrol.Error.ComplexBashDetected`         |
//! | `PolkitDenied`          | `org.bootcontrol.Error.PolkitDenied`                |
//! | `EspScanFailed`         | `org.bootcontrol.Error.EspScanFailed`               |
//! | `SecurityPolicyViolation` | `org.bootcontrol.Error.SecurityPolicyViolation`   |
//! | `ConcurrentModification`| `org.bootcontrol.Error.ConcurrentModification`     |
//! | `ToolNotFound`          | `org.bootcontrol.Error.ToolNotFound`               |
//! | `NvramBackupFailed`     | `org.bootcontrol.Error.NvramBackupFailed`          |
//! | `MokKeyNotFound`        | `org.bootcontrol.Error.MokKeyNotFound`             |
//! | `SigningFailed`         | `org.bootcontrol.Error.SigningFailed`              |
//! | `SnapshotNotFound`      | `org.bootcontrol.Error.SnapshotNotFound`           |
//! | `SnapshotCorrupt`       | `org.bootcontrol.Error.SnapshotCorrupt`            |
//! | `SnapshotFailed`        | `org.bootcontrol.Error.SnapshotFailed`             |

use bootcontrol_core::error::BootControlError;
use zbus::DBusError;

use crate::snapshot::SnapshotError;

/// Typ błędu D-Bus dla metod interfejsu `org.bootcontrol.Manager`.
///
/// Każdy wariant jest mapowany na strukturalną nazwę D-Bus przez
/// `#[derive(DBusError)]` z prefixem `org.bootcontrol.Error`.
/// Klienci przechwytują **nazwę błędu**, nigdy nie parsują wiadomości.
///
/// Wariant `ZBus` jest wymagany przez derive macro — obsługuje wewnętrzne
/// błędy transportu D-Bus, które nie są błędami aplikacji.
#[derive(Debug, DBusError)]
#[zbus(prefix = "org.bootcontrol.Error")]
pub enum DaemonError {
    /// Wewnętrzny błąd transportu zbus. Wymagany przez `#[derive(DBusError)]`.
    #[zbus(error)]
    ZBus(zbus::Error),

    /// Wersja ETag podana przez klienta nie zgadza się z plikiem na dysku.
    StateMismatch(String),

    /// Żądany klucz nie istnieje w konfiguracji.
    KeyNotFound(String),

    /// Wartość klucza nie może być sparsowana do oczekiwanego typu.
    MalformedValue(String),

    /// Plik zawiera konstrukcje Bash niemożliwe do bezpiecznego parsowania.
    ComplexBashDetected(String),

    /// Żądanie zapisu odrzucone przez Polkit.
    PolkitDenied(String),

    /// Błąd I/O podczas odczytu/zapisu ESP lub `/etc/default/grub`.
    EspScanFailed(String),

    /// Payload narusza politykę bezpieczeństwa (blacklista).
    SecurityPolicyViolation(String),

    /// Inny proces trzyma wyłączny lock na pliku (`flock EWOULDBLOCK`).
    ConcurrentModification(String),

    /// Wymagane narzędzie zewnętrzne nie znaleziono na `$PATH`.
    ToolNotFound(String),

    /// Backup zmiennych EFI NVRAM nie powiódł się.
    NvramBackupFailed(String),

    /// Klucz MOK lub certyfikat nie znaleziono pod oczekiwaną ścieżką.
    MokKeyNotFound(String),

    /// Operacja podpisywania lub rejestracji certyfikatu zakończyła się błędem.
    SigningFailed(String),

    /// Operacja generowania kluczy nie powiodła się.
    KeyGenerationFailed(String),

    /// Zapis zmiennej EFI NVRAM nie powiódł się.
    NvramWriteFailed(String),

    /// Wskazany snapshot nie istnieje pod katalogiem snapshot root.
    SnapshotNotFound(String),

    /// Manifest snapshotu jest niespójny lub w wersji nowszej niż obsługiwana.
    SnapshotCorrupt(String),

    /// Operacja snapshotu (create / list / restore) nie powiodła się z innego powodu.
    SnapshotFailed(String),

    /// Nieznany błąd — catch-all dla wariantów z `#[non_exhaustive]`.
    Failed(String),
}

/// Przekształć [`SnapshotError`] na [`DaemonError`] z poprawną nazwą D-Bus.
///
/// Wszystkie warianty `SnapshotError` mapują się na trzy publiczne nazwy
/// D-Bus: `SnapshotNotFound`, `SnapshotCorrupt`, `SnapshotFailed`.
/// GUI rozpoznaje błąd po nazwie wariantu, nie po treści wiadomości.
pub fn snapshot_to_daemon_error(e: SnapshotError) -> DaemonError {
    let msg = e.to_string();
    match e {
        SnapshotError::NotFound(_) => DaemonError::SnapshotNotFound(msg),
        SnapshotError::SchemaUpgradeRequired(_) => DaemonError::SnapshotCorrupt(msg),
        SnapshotError::Serde(_) => DaemonError::SnapshotCorrupt(msg),
        SnapshotError::Io(_) => DaemonError::SnapshotFailed(msg),
    }
}

/// Przekształć [`BootControlError`] na [`DaemonError`] z poprawną nazwą D-Bus.
///
/// Wiadomość tekstowa pochodzi z `Display` implementacji `BootControlError`
/// — brak duplikacji stringów. Klienci identyfikują błąd po nazwie wariantu,
/// nie po treści wiadomości.
///
/// # Arguments
///
/// * `e` — Błąd aplikacji do przekształcenia.
///
/// # Examples
///
/// ```
/// use bootcontrol_core::error::BootControlError;
/// use bootcontrold::dbus_error::{to_daemon_error, DaemonError};
///
/// let err = BootControlError::PolkitDenied;
/// let daemon_err = to_daemon_error(err);
/// assert!(matches!(daemon_err, DaemonError::PolkitDenied(_)));
/// ```
pub fn to_daemon_error(e: BootControlError) -> DaemonError {
    let msg = e.to_string();
    match e {
        BootControlError::StateMismatch { .. } => DaemonError::StateMismatch(msg),
        BootControlError::KeyNotFound { .. } => DaemonError::KeyNotFound(msg),
        BootControlError::MalformedValue { .. } => DaemonError::MalformedValue(msg),
        BootControlError::ComplexBashDetected { .. } => DaemonError::ComplexBashDetected(msg),
        BootControlError::PolkitDenied => DaemonError::PolkitDenied(msg),
        BootControlError::EspScanFailed { .. } => DaemonError::EspScanFailed(msg),
        BootControlError::SecurityPolicyViolation { .. } => {
            DaemonError::SecurityPolicyViolation(msg)
        }
        BootControlError::ConcurrentModification { .. } => DaemonError::ConcurrentModification(msg),
        BootControlError::ToolNotFound { .. } => DaemonError::ToolNotFound(msg),
        BootControlError::NvramBackupFailed { .. } => DaemonError::NvramBackupFailed(msg),
        BootControlError::MokKeyNotFound { .. } => DaemonError::MokKeyNotFound(msg),
        BootControlError::SigningFailed { .. } => DaemonError::SigningFailed(msg),
        BootControlError::KeyGenerationFailed { .. } => DaemonError::KeyGenerationFailed(msg),
        BootControlError::NvramWriteFailed { .. } => DaemonError::NvramWriteFailed(msg),
        // #[non_exhaustive] — przyszłe warianty mapują na Failed
        _ => DaemonError::Failed(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_mismatch_maps_to_correct_variant() {
        let e = BootControlError::StateMismatch {
            expected: "aabb".into(),
            actual: "ccdd".into(),
        };
        assert!(matches!(to_daemon_error(e), DaemonError::StateMismatch(_)));
    }

    #[test]
    fn key_not_found_maps_to_correct_variant() {
        let e = BootControlError::KeyNotFound { key: "X".into() };
        assert!(matches!(to_daemon_error(e), DaemonError::KeyNotFound(_)));
    }

    #[test]
    fn malformed_value_maps_to_correct_variant() {
        let e = BootControlError::MalformedValue {
            key: "X".into(),
            reason: "r".into(),
        };
        assert!(matches!(to_daemon_error(e), DaemonError::MalformedValue(_)));
    }

    #[test]
    fn complex_bash_detected_maps_to_correct_variant() {
        let e = BootControlError::ComplexBashDetected {
            offender: "sub".into(),
        };
        assert!(matches!(
            to_daemon_error(e),
            DaemonError::ComplexBashDetected(_)
        ));
    }

    #[test]
    fn polkit_denied_maps_to_correct_variant() {
        assert!(matches!(
            to_daemon_error(BootControlError::PolkitDenied),
            DaemonError::PolkitDenied(_)
        ));
    }

    #[test]
    fn esp_scan_failed_maps_to_correct_variant() {
        let e = BootControlError::EspScanFailed {
            reason: "io".into(),
        };
        assert!(matches!(to_daemon_error(e), DaemonError::EspScanFailed(_)));
    }

    #[test]
    fn security_policy_violation_maps_to_correct_variant() {
        let e = BootControlError::SecurityPolicyViolation {
            reason: "init=".into(),
        };
        assert!(matches!(
            to_daemon_error(e),
            DaemonError::SecurityPolicyViolation(_)
        ));
    }

    #[test]
    fn concurrent_modification_maps_to_correct_variant() {
        let e = BootControlError::ConcurrentModification {
            path: "/etc/default/grub".into(),
        };
        assert!(matches!(
            to_daemon_error(e),
            DaemonError::ConcurrentModification(_)
        ));
    }

    #[test]
    fn tool_not_found_maps_to_correct_variant() {
        let e = BootControlError::ToolNotFound {
            tool: "sbsign".into(),
        };
        assert!(matches!(to_daemon_error(e), DaemonError::ToolNotFound(_)));
    }

    #[test]
    fn nvram_backup_failed_maps_to_correct_variant() {
        let e = BootControlError::NvramBackupFailed {
            reason: "no variables".into(),
        };
        assert!(matches!(
            to_daemon_error(e),
            DaemonError::NvramBackupFailed(_)
        ));
    }

    #[test]
    fn mok_key_not_found_maps_to_correct_variant() {
        let e = BootControlError::MokKeyNotFound {
            path: "/var/lib/bootcontrol/keys/mok.key".into(),
        };
        assert!(matches!(to_daemon_error(e), DaemonError::MokKeyNotFound(_)));
    }

    #[test]
    fn signing_failed_maps_to_correct_variant() {
        let e = BootControlError::SigningFailed {
            reason: "sbsign exited 1".into(),
        };
        assert!(matches!(to_daemon_error(e), DaemonError::SigningFailed(_)));
    }

    #[test]
    fn key_generation_failed_maps_to_correct_variant() {
        let e = BootControlError::KeyGenerationFailed {
            reason: "openssl failed".into(),
        };
        assert!(matches!(
            to_daemon_error(e),
            DaemonError::KeyGenerationFailed(_)
        ));
    }

    #[test]
    fn nvram_write_failed_maps_to_correct_variant() {
        let e = BootControlError::NvramWriteFailed {
            reason: "efivar error".into(),
        };
        assert!(matches!(
            to_daemon_error(e),
            DaemonError::NvramWriteFailed(_)
        ));
    }

    #[test]
    fn snapshot_not_found_maps_to_correct_variant() {
        let e = SnapshotError::NotFound("2026-04-30T130211Z-test_op".into());
        assert!(matches!(
            snapshot_to_daemon_error(e),
            DaemonError::SnapshotNotFound(_)
        ));
    }

    #[test]
    fn snapshot_schema_upgrade_maps_to_corrupt() {
        let e = SnapshotError::SchemaUpgradeRequired(42);
        assert!(matches!(
            snapshot_to_daemon_error(e),
            DaemonError::SnapshotCorrupt(_)
        ));
    }

    #[test]
    fn snapshot_io_maps_to_failed() {
        let e = SnapshotError::Io(std::io::Error::other("disk full"));
        let mapped = snapshot_to_daemon_error(e);
        assert!(matches!(mapped, DaemonError::SnapshotFailed(_)));
    }

    #[test]
    fn snapshot_serde_maps_to_corrupt() {
        // Force a serde_json error by parsing invalid JSON.
        let bad: serde_json::Error = serde_json::from_str::<serde_json::Value>("not json")
            .err()
            .unwrap();
        let e = SnapshotError::Serde(bad);
        assert!(matches!(
            snapshot_to_daemon_error(e),
            DaemonError::SnapshotCorrupt(_)
        ));
    }

    #[test]
    fn daemon_error_message_contains_original_description() {
        let e = BootControlError::StateMismatch {
            expected: "aabb".into(),
            actual: "ccdd".into(),
        };
        let original_msg = e.to_string();
        let daemon_err = to_daemon_error(BootControlError::StateMismatch {
            expected: "aabb".into(),
            actual: "ccdd".into(),
        });
        let DaemonError::StateMismatch(msg) = daemon_err else {
            panic!("wrong variant");
        };
        assert_eq!(msg, original_msg);
    }

    #[test]
    fn all_known_variants_produce_non_failed_daemon_error() {
        let cases: Vec<(&str, DaemonError)> = vec![
            (
                "StateMismatch",
                to_daemon_error(BootControlError::StateMismatch {
                    expected: "a".into(),
                    actual: "b".into(),
                }),
            ),
            (
                "KeyNotFound",
                to_daemon_error(BootControlError::KeyNotFound { key: "K".into() }),
            ),
            (
                "MalformedValue",
                to_daemon_error(BootControlError::MalformedValue {
                    key: "K".into(),
                    reason: "r".into(),
                }),
            ),
            (
                "ComplexBashDetected",
                to_daemon_error(BootControlError::ComplexBashDetected {
                    offender: "s".into(),
                }),
            ),
            (
                "PolkitDenied",
                to_daemon_error(BootControlError::PolkitDenied),
            ),
            (
                "EspScanFailed",
                to_daemon_error(BootControlError::EspScanFailed {
                    reason: "io".into(),
                }),
            ),
            (
                "SecurityPolicyViolation",
                to_daemon_error(BootControlError::SecurityPolicyViolation { reason: "i".into() }),
            ),
            (
                "ConcurrentModification",
                to_daemon_error(BootControlError::ConcurrentModification { path: "/p".into() }),
            ),
            (
                "ToolNotFound",
                to_daemon_error(BootControlError::ToolNotFound { tool: "sbsign".into() }),
            ),
            (
                "NvramBackupFailed",
                to_daemon_error(BootControlError::NvramBackupFailed {
                    reason: "test".into(),
                }),
            ),
            (
                "MokKeyNotFound",
                to_daemon_error(BootControlError::MokKeyNotFound {
                    path: "/var/lib/bootcontrol/keys/mok.key".into(),
                }),
            ),
            (
                "SigningFailed",
                to_daemon_error(BootControlError::SigningFailed {
                    reason: "sbsign exited 1".into(),
                }),
            ),
            (
                "KeyGenerationFailed",
                to_daemon_error(BootControlError::KeyGenerationFailed {
                    reason: "openssl failed".into(),
                }),
            ),
            (
                "NvramWriteFailed",
                to_daemon_error(BootControlError::NvramWriteFailed {
                    reason: "efivar error".into(),
                }),
            ),
        ];

        for (name, err) in cases {
            assert!(
                !matches!(err, DaemonError::Failed(_)),
                "Wariant {name} zmapował się na DaemonError::Failed — zaktualizuj to_daemon_error()"
            );
        }
    }
}
