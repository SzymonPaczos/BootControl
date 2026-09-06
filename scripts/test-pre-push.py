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
        self.base = self.commit()
        self.git('push', '-q', 'origin', 'main')

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

if __name__ == '__main__':
    unittest.main()
