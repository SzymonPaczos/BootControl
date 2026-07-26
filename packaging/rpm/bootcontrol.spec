Name:           bootcontrol
Version:        0.1.0
Release:        1%{?dist}
Summary:        A modern, memory-safe bootloader manager for Linux — built in Rust

License:        GPL-3.0-only
URL:            https://github.com/szymonpaczos/bootcontrol
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  dbus-devel
BuildRequires:  systemd-rpm-macros

Requires:       dbus
Requires:       polkit

%description
BootControl is a declarative bootloader configuration manager that replaces
tools like Grub Customizer. It uses native parsers (no Bash injection),
requires Polkit authorization for every write operation, and employs ETag-based
optimistic concurrency control to guarantee safe, atomic updates to boot files.

This package provides the unprivileged user-facing binary (CLI, TUI, GUI
frontends). The privileged backend daemon is in the bootcontrold sub-package.

# ---------------------------------------------------------------------------
# Sub-package: bootcontrold
# ---------------------------------------------------------------------------

%package -n bootcontrold
Summary:        BootControl privileged D-Bus daemon
Requires:       dbus
Requires:       polkit
Requires:       systemd

%description -n bootcontrold
The socket-activated, privilege-separated daemon that backs all BootControl
frontends. It runs as root, is activated on demand through D-Bus and systemd,
and shuts down
after 60 seconds of inactivity. All write operations are authorized via Polkit
and protected by SHA-256 ETag concurrency checks.

This package contains:
  - The bootcontrold daemon binary
  - systemd service and compatibility socket unit
  - D-Bus system bus policy (org.bootcontrol.Manager)
  - Polkit action policy (four active per-intent actions: rewrite-grub, write-bootloader, enroll-mok, restore-snapshot)

# ---------------------------------------------------------------------------
# %prep
# ---------------------------------------------------------------------------

%prep
%autosetup -n %{name}-%{version}

# ---------------------------------------------------------------------------
# %build
# ---------------------------------------------------------------------------

%build
cargo build --release --workspace

# ---------------------------------------------------------------------------
# %install
# ---------------------------------------------------------------------------

%install
# User-facing binary
install -Dm755 target/release/bootcontrol \
    %{buildroot}%{_bindir}/bootcontrol

# Daemon binary
install -Dm755 target/release/bootcontrold \
    %{buildroot}%{_bindir}/bootcontrold

# systemd unit and socket files
install -Dm644 packaging/systemd/bootcontrold.service \
    %{buildroot}%{_unitdir}/bootcontrold.service
install -Dm644 packaging/systemd/bootcontrold.socket \
    %{buildroot}%{_unitdir}/bootcontrold.socket

# D-Bus system bus policy
install -Dm644 packaging/dbus/org.bootcontrol.Manager.conf \
    %{buildroot}%{_datadir}/dbus-1/system.d/org.bootcontrol.Manager.conf
install -Dm644 packaging/dbus/org.bootcontrol.Manager.service \
    %{buildroot}%{_datadir}/dbus-1/system-services/org.bootcontrol.Manager.service

# Polkit action policy
install -Dm644 packaging/polkit/org.bootcontrol.policy \
    %{buildroot}%{_datadir}/polkit-1/actions/org.bootcontrol.policy

# GRUB failsafe menu hook
install -Dm755 packaging/grub.d/40_bootcontrol \
    %{buildroot}%{_sysconfdir}/grub.d/40_bootcontrol

# ---------------------------------------------------------------------------
# %files — main package (bootcontrol)
# ---------------------------------------------------------------------------

%files
%license LICENSE
%doc README.md
%{_bindir}/bootcontrol

# ---------------------------------------------------------------------------
# %files — sub-package (bootcontrold)
# ---------------------------------------------------------------------------

%files -n bootcontrold
%license LICENSE
%{_bindir}/bootcontrold
%{_unitdir}/bootcontrold.service
%{_unitdir}/bootcontrold.socket
%{_datadir}/dbus-1/system.d/org.bootcontrol.Manager.conf
%{_datadir}/dbus-1/system-services/org.bootcontrol.Manager.service
%{_datadir}/polkit-1/actions/org.bootcontrol.policy
%config %{_sysconfdir}/grub.d/40_bootcontrol

# ---------------------------------------------------------------------------
# systemd scriptlets for bootcontrold
# ---------------------------------------------------------------------------

%post -n bootcontrold
%systemd_post bootcontrold.service

%preun -n bootcontrold
%systemd_preun bootcontrold.service

%postun -n bootcontrold
%systemd_postun_with_restart bootcontrold.service

# ---------------------------------------------------------------------------
# %changelog
# ---------------------------------------------------------------------------

%changelog
* Thu Apr 17 2026 Szymon Paczos <szymon@example.com> - 0.1.0-1
- Initial RPM packaging
