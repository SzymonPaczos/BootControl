#!/usr/bin/env python3
"""Check audit provenance in a commit, not in the caller's working tree.

This validates evidence presence and ancestry, not the truth of a review or
whether all its findings are fixed. Existing FAIL findings remain in backlog.
"""
from datetime import date
import re
import subprocess
import sys


def parse_evidence(text):
    """Return the unique evidence fields from the newest audit entry.

    Raises ValueError for missing/duplicate fields, stale dates, placeholders,
    unpinned revisions or a NOT_DUE verdict without an explanation.
    """
    headings = list(re.finditer(r'^## Audyt (\d{4}-\d{2}-\d{2})(?:[^\n]*)$', text, re.M))
    if not headings:
        raise ValueError('missing audit heading')
    age = (date.today() - date.fromisoformat(headings[0][1])).days
    if not 0 <= age <= 7:
        raise ValueError('latest audit is not within the last seven days')
    entry = text[headings[0].end():headings[1].start() if len(headings) > 1 else len(text)]
    fields = {}
    for key in ['AUDITED_REVISION', 'SECURITY_REVIEW', 'RED_TEAM']:
        matches = re.findall(r'^' + key + r':[ \t]*(.*)$', entry, re.M)
        if len(matches) != 1 or not matches[0].strip():
            raise ValueError(f'{key}: expected one nonempty field in the latest audit')
        fields[key] = matches[0].strip()
    revision = fields['AUDITED_REVISION'].split()[0]
    if not re.fullmatch(r'[0-9a-f]{40}|[0-9a-f]{64}', revision):
        raise ValueError('AUDITED_REVISION must pin a full commit SHA')
    fields['AUDITED_REVISION'] = revision
    for key in ['SECURITY_REVIEW', 'RED_TEAM']:
        value = fields[key]
        if not re.match(r'^(PASS|FAIL|FINDINGS)(?:\b|$)|^NOT_DUE\s+\S', value):
            raise ValueError(f'{key}: missing completed verdict or explained NOT_DUE')
    return fields


def main():
    """Exit nonzero on invalid evidence, missing objects or Git errors."""
    if len(sys.argv) != 2 or not re.fullmatch(r'[0-9a-f]{40}|[0-9a-f]{64}', sys.argv[1]):
        raise ValueError('usage: check-audit-evidence.py <full pushed SHA>')
    tip = sys.argv[1]
    text = subprocess.check_output(
        ['git', 'show', f'{tip}:.claude/audit-log.md'], text=True)
    fields = parse_evidence(text)
    audited = fields['AUDITED_REVISION']
    subprocess.run(['git', 'cat-file', '-e', f'{audited}^{{commit}}'], check=True)
    subprocess.run(['git', 'merge-base', '--is-ancestor', audited, tip], check=True)
    print(f'pre-push: audit evidence pins reachable commit {audited}')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f'pre-push: invalid audit evidence: {error}', file=sys.stderr)
        sys.exit(1)
