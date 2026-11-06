---
paths:
  - "crates/core/**"
  - "crates/daemon/**"
  - "crates/client/**"
  - "packaging/**"
---
# BootControl — granica uprzywilejowana

Przed zmianą przeczytaj właściwy kontrakt ARCHITECTURE.md oraz
`../conventions/bootcontrol-development.md`; pełne decyzje w rejestrze.

- Daemon on-demand/socket activation, IdleTimeoutSec=60; żadnego boot dependency.
- Polkit przed wszelkim disk I/O; 4 per-intent:
  org.bootcontrol.rewrite-grub, write-bootloader, enroll-mok, restore-snapshot.
  Źródło: packaging/polkit/org.bootcontrol.policy + daemon/src/polkit.rs;
  policy_check.rs odrzuca manage-only policy. Nie wracają usunięte akcje paranoia.
- Każdy write rewaliduje payload i ETag, blokuje flock(LOCK_EX|LOCK_NB),
  zapisuje .tmp → fsync → atomic rename; fail/lock mismatch przerywa operację.
- Jedna core::security::KERNEL_CMDLINE_BLACKLIST; daemon::sanitize::check_payload
  i backends::uki::validate_kernel_param muszą jej używać. init=, selinux=0,
  apparmor=0 nie przechodzą; frontend validation nie zastępuje daemona.
- Preflight /etc/os-release: NixOS refuse; ostree rpm-ostree kargs nadal
  sanitizowane; ESP tylko bieżącej sygnatury OS. Bez operacji na obcym Linuksie.
- Secure Boot offline: lokalne efivars, backup /var/lib/bootcontrol/certs;
  bez reqwest/curl/hyper, pobieranych certów, private keys w Git.
- TDD przed produkcją, pure &str parsers, mutacje testowane przez tempfile,
  komentarze configów zachowane bajtowo; unsafe wymaga SAFETY.
