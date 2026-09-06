#!/usr/bin/env python3
"""Run toolkit's six unchanged assertions with BootControl prerequisite fixtures.

The generic toolkit fixture has no audit or CI script. Supply those inputs,
including a tiny fixture-only secret check, without changing the hook or tests.
Usage: python3 scripts/test-toolkit-push-gates.py /path/to/toolkit
"""
from pathlib import Path
import subprocess
import sys
import tempfile

source = Path(sys.argv[1]) / 'templates/test-gates.sh'
hook = Path(__file__).resolve().parents[1] / '.githooks/pre-push'
fixture = r'''mkdir -p .githooks .claude scripts
        printf '## Audyt %s\n' "$(date +%F)" > .claude/audit-log.md
        cat > scripts/ci-local.sh <<'CI'
#!/usr/bin/env bash
set -eu
printf 'TODO/FIXME: fixture advisory layer reached\n'
if git grep -nE '^(api_key|secret) = "' -- app.js cfg.py; then exit 1; fi
CI
        chmod +x scripts/ci-local.sh'''
text = source.read_text()
assert text.count('mkdir -p .githooks') == 1, 'Toolkit fixture changed; review adapter'
text = text.replace('mkdir -p .githooks', fixture)
with tempfile.TemporaryDirectory(prefix='bootcontrol-toolkit-gates-') as directory:
    script = Path(directory) / 'test-gates.sh'
    script.write_text(text)
    sys.exit(subprocess.run(['bash', str(script), str(hook)]).returncode)
