#!/usr/bin/env python3
"""Exercise the real hook in disposable repositories with a local bare remote."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from datetime import date

HOOK = Path(__file__).resolve().parents[1] / '.githooks/pre-push'
ZERO = '0' * 40

class PushGate(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='bootcontrol-gate-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / 'repo with spaces'
        self.repo.mkdir()
        self.env = os.environ.copy()
        for name in list(self.env):
            if name.startswith('GIT_'):
                self.env.pop(name)
        self.env['GATE_TRACE'] = str(self.root / 'trace')
        self.git('init', '-q', '--initial-branch=main')
        self.git('config', 'user.name', 'Gate Test')
        self.git('config', 'user.email', 'gate@example.invalid')
        self.git('config', 'core.hooksPath', '/dev/null')
        subprocess.run(['git', 'init', '-q', '--bare', str(self.root/'remote.git')], check=True, env=self.env)
        self.git('remote', 'add', 'origin', str(self.root/'remote.git'))
        (self.repo / '.githooks').mkdir()
        shutil.copy(HOOK, self.repo / '.githooks/pre-push')
        (self.repo / 'scripts').mkdir()
        (self.repo / '.claude').mkdir()
        (self.repo / '.claude/audit-log.md').write_text(f'## Audyt {date.today()}\n')
        self.ci = self.repo / 'scripts/ci-local.sh'
        self.ci.write_text('#!/usr/bin/env bash\nset -eu\nprintf "%s\\n" "$(git rev-parse HEAD)" >> "$GATE_TRACE"\n! grep -q BAD payload\n')
        self.ci.chmod(0o755)
        (self.repo / 'payload').write_text('GOOD\n')
        checker = HOOK.parents[1] / 'scripts/check-audit-evidence.py'
        if checker.exists():
            shutil.copy(checker, self.repo / 'scripts/check-audit-evidence.py')
        self.audited = self.commit()
        self.write_audit()
        self.base = self.commit()
        self.git('push', '-q', 'origin', 'main')

    def write_audit(self, **overrides):
        fields = {'AUDITED_REVISION': self.audited,
                  'SECURITY_REVIEW': 'PASS (fixture review)',
                  'RED_TEAM': 'NOT_DUE (fixture scope unchanged)'}
        fields.update(overrides)
        (self.repo/'.claude/audit-log.md').write_text(
            f'## Audyt {date.today()}\n\n```text\n' +
            ''.join(f'{key}: {value}\n' for key, value in fields.items()) + '```\n')

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.repo, env=self.env, stderr=subprocess.STDOUT, text=True).strip()

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'test: fixture')
        return self.git('rev-parse', 'HEAD')

    def run_hook(self, rows):
        result = subprocess.run(['bash', str(HOOK), 'origin', str(self.root/'remote.git')], cwd=self.repo, env=self.env, input=rows, text=True, capture_output=True)
        self.assertEqual(self.git('worktree', 'list', '--porcelain').count('worktree '), 1, result.stderr)
        return result

    def row(self, sha, remote=None, ref='main'):
        return f'refs/heads/{ref} {sha} refs/heads/{ref} {remote or self.base}\n'

    def test_dirty_fix_cannot_hide_bad_commit(self):
        (self.repo/'payload').write_text('BAD\n')
        sha = self.commit()
        (self.repo/'payload').write_text('GOOD\n')
        result = self.run_hook(self.row(sha))
        self.assertNotEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assertEqual((self.repo/'payload').read_text(), 'GOOD\n')

    def test_dirty_breakage_does_not_fail_good_commit(self):
        (self.repo/'payload').write_text('BAD\n')
        result = self.run_hook(self.row(self.base))
        self.assertEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assertEqual((self.root/'trace').read_text().splitlines(), [self.base])

    def test_new_branch_and_multiple_refs(self):
        (self.repo/'payload').write_text('BAD\n')
        bad = self.commit()
        self.git('checkout', '-q', self.base)
        result = self.run_hook(self.row(self.base)+self.row(bad, ZERO, 'new'))
        self.assertNotEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assertEqual((self.root/'trace').read_text().splitlines(), [self.base, bad])

    def test_delete_and_empty_push_run_no_ci(self):
        self.assertEqual(self.run_hook(self.row(ZERO)).returncode, 0)
        self.assertEqual(self.run_hook('').returncode, 0)
        self.assertFalse((self.root/'trace').exists())

    def test_bad_input_and_missing_object_fail_closed(self):
        for data in ['garbage\n', 'truncated', self.row('f'*40), self.row(self.base).strip()+' extra\n']:
            self.assertNotEqual(self.run_hook(data).returncode, 0, data)

    def test_runner_failure_and_missing_runner_fail_closed(self):
        self.ci.write_text('#!/bin/sh\nexit 42\n')
        sha = self.commit()
        self.assertNotEqual(self.run_hook(self.row(sha)).returncode, 0)
        self.ci.unlink()
        sha = self.commit()
        self.assertNotEqual(self.run_hook(self.row(sha)).returncode, 0)

    def test_audit_must_come_from_pushed_commit(self):
        audit = self.repo/'.claude/audit-log.md'
        audit.write_text('## Audyt 2000-01-01\n')
        sha = self.commit()
        audit.write_text(f'## Audyt {date.today()}\n')
        self.assertNotEqual(self.run_hook(self.row(sha)).returncode, 0)

    def test_real_push_to_local_remote_rejects_bad_commit(self):
        self.git('config', 'core.hooksPath', '.githooks')
        (self.repo/'payload').write_text('BAD\n')
        self.commit()
        (self.repo/'payload').write_text('GOOD\n')
        result = subprocess.run(['git', 'push', 'origin', 'main'], cwd=self.repo, env=self.env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assertEqual(self.git('rev-parse', 'origin/main'), self.base)

    def test_date_alone_is_not_audit_evidence(self):
        (self.repo/'.claude/audit-log.md').write_text(f'## Audyt {date.today()}\n')
        self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0)

    def test_unreachable_or_unknown_audited_revision_is_rejected(self):
        orphan = self.git('commit-tree', self.git('rev-parse', 'HEAD^{tree}'), '-m', 'unrelated')
        for revision in [orphan, 'f'*40, 'HEAD', 'main']:
            self.write_audit(AUDITED_REVISION=revision)
            self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0, revision)

    def test_placeholder_or_absent_review_is_rejected(self):
        for field in ['SECURITY_REVIEW', 'RED_TEAM']:
            for value in ['', 'TODO', 'n/a', 'NOT_DUE']:
                self.write_audit(**{field: value})
                self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0, (field, value))

    def test_older_entry_cannot_supply_missing_current_evidence(self):
        audit = self.repo/'.claude/audit-log.md'
        audit.write_text(f'## Audyt {date.today()}\nno review\n\n' + audit.read_text())
        self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0)

    def test_working_tree_review_cannot_complete_pushed_audit(self):
        self.write_audit(SECURITY_REVIEW='')
        sha = self.commit()
        self.write_audit()
        self.assertNotEqual(self.run_hook(self.row(sha)).returncode, 0)

    def test_duplicate_evidence_fields_are_rejected(self):
        audit = self.repo/'.claude/audit-log.md'
        audit.write_text(audit.read_text() + f'AUDITED_REVISION: {self.audited}\n')
        self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0)

    def test_symlinked_ci_cannot_read_uncommitted_runner(self):
        external = self.root/'uncommitted-runner'
        external.write_text('#!/bin/sh\nexit 0\n')
        external.chmod(0o755)
        self.ci.unlink()
        self.ci.symlink_to(external)
        self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0)

    def test_symlinked_audit_is_not_committed_evidence(self):
        audit = self.repo/'.claude/audit-log.md'
        external = self.root/'uncommitted-audit'
        external.write_text(audit.read_text())
        audit.unlink()
        audit.symlink_to(external)
        self.assertNotEqual(self.run_hook(self.row(self.commit())).returncode, 0)

if __name__ == '__main__':
    unittest.main()
