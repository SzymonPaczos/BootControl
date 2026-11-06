# Run blockers and unverified facts — incomplete run

No independently validated needs_validation finding exists in findings.json yet.
The following are RUN-LEVEL gaps, not vulnerability records and carry no severity.

1. Explicit owner endpoint consent: the root has asked whether the exact public
   BootControl bd28f2d snapshot may be processed by Claude/Anthropic using
   claude-fable-5-1. Until the actual answer arrives, do not start recon-d, hunters,
   critics or verifiers; no alternate transport or indirect execution is permitted.
2. Baseline completeness: recon-d (local execution/deployment visibility) has not run.
   The other three roles returned facts only. Their unread source regions must be
   mapped before hunting; no “PASS target” follows from transport PASS.
3. Native runtime: the approved isolated executor lacks a Rust toolchain. A candidate
   whose decisive fact needs Rust/Linux behavior must receive a genuine source trace,
   exact blocker and bounded dummy-fixture plan through independent candidate and
   final-record verification. Do not replace Rust execution with a Python imitation.
4. Deployment facts: system Polkit/agent behavior, installed release feature selection,
   directory/file ownership, effective systemd sandbox and firmware/boot behavior are
   not established by source alone. Owner-observed configuration checks and bounded
   offline VM fixtures may resolve relevant facts, without live probes or real keys.
5. Meta-test scope: the separate10/10 audit-wrapper result uses explicit adoption
   overlays. It is not a native app/security result for frozen bd28 and cannot confirm
   a GRUB, snapshot, authorization or firmware vulnerability claim.

All 23 in-scope units are deferred in coverage-ledger.json. No unvalidated hypothesis
has been promoted to findings.json, assigned a severity or called a confirmed exploit.
REPORT.md and run-metadata.json state the exact incomplete reason and resume steps.
