# BootControl frozen-source architecture — provisional Phase 1

Source: bd28f2d0ce55b5a869c097e60429a41bf4240e63, clean frozen checkout.
Profile standard; no strict total invocation budget. This architecture is provisional
until fourth baseline reconnaissance d completes. No hunter coverage is claimed.

BootControl is a Linux boot-configuration manager, not a boot dependency. A user-space
CLI/TUI/Slint GUI communicates through bootcontrol-client and zbus with bootcontrold,
a root system D-Bus service. Lower-trust actors include another local OS user/bus peer,
a client with a deliberately limited per-intent Polkit grant, mutable boot/configuration
content and filesystem objects at managed paths. Administrative access by itself is
not an exploit. The principal protected resources are /etc/default/grub, bootloader
configuration/entries, EFI variables, snapshot state, firmware-signing keys and shared
daemon availability. Grub Customizer and efibootmgr-like variable management are
source-visible comparables, not evidence that unsafe behavior is acceptable.

Rust2021 Cargo workspace: pure core parsing/ETag helpers, privileged daemon filesystem
and process adapters, client DTO/backend layer and frontends. Source declares Linux-only
daemon and E2E targets. Windows-aware UEFI/frontend code is reviewed only for visible
boundaries; no Windows/UAC or firmware runtime guarantee is made. Packaging contains
AUR/RPM, D-Bus/polkit/systemd files and GRUB hook. No hosted CI; opt-in local hooks run
ci-local against a pushed commit. The root orchestrator separately supplies dependency,
CVE/KEV and secret-scan receipts; those are not duplicated as exploit hunting.

Principal entry surface is org.bootcontrol.Manager. Writes resolve the unique sender
and Polkit system-bus-name subject, enforce one of four per-intent actions and host
preflight, then use backend-specific validation and version/locking rules. Reads lack
Polkit checks and therefore require explicit protected-data and caller-scope review.
Source-visible controls include sender-derived identity, KNOWN_ACTIONS/policy consistency,
immutable-host handling, ETag+flock, parsed configuration restrictions, managed restore
paths and UKI ownership/token checks. Their presence does not prove every parallel
backend or race/error path satisfies the same invariant.

Critical flows: arbitrary GRUB key/value serialization → atomic rewrite → grub-mkconfig;
typed settings/default selection → version checks → snapshot callback → rewrite;
RestoreSnapshot → manifest/path validation → current state/locks → replacement;
loader/kernel/EFI mutations; NVRAM backup source/destination confinement; managed UKI
validation → signing/enrollment; external helper selection; confirmation/staging and
daemon cancellation/idle lifetime. Recon facts are starting points, not findings.

Selected companion domains: local IPC for caller/helper/file authority; native/binary
for EFI representations, unsafe/ABI and executable identity; data lifecycle for
snapshots/restore/backups; resource availability for shared root-daemon lifecycle;
supply chain for package feature/install and local gate authority; RPC for identity and
method/DTO binding. HTTP/browser/cloud-provider/AI product boundaries are not inferred
from incidental dependencies. UI cosmetics, gui-spike prototypes and reference art are
excluded; trust-sensitive frontend confirmation and rescue remain in scope.

No compatible prior JSON coverage/findings exists. Historical prose remains context,
not carried confirmations or a reason to suppress units. Every planned unit has exact
source-derived canonical identity and a block map. Fresh hunters, immediate wave critic,
separate final critic, candidate validation and final record verification remain required.

Execution: researchers use only Read/Glob/Grep in a copied public committed-source packet,
without Git metadata, secret filename patterns, ignored data or host profiles. No target
build/tests/processes are executed by model researchers. Root has a separate OS-enforced
executor: actual capability/receipt review is pending; Rust compiler/native Linux facts
are presently unavailable. Any decisive unavailable fact must remain needs_validation
without severity. No deployed endpoints, credentials, real firmware or shared services
are in scope. Public-source export was separately proved; an explicit endpoint-consent
question is pending, so no new external call starts until owner response is relayed.
