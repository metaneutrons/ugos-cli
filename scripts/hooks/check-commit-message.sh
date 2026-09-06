#!/bin/sh
# Checks a commit message against Conventional Commits and forbids attribution
# trailers from AI tooling.
#
# Usage: check-commit-message.sh <path-to-message>
# Manager-independent. Works as a commit-msg hook under lefthook, husky,
# pre-commit or directly under core.hooksPath.
set -eu

msg_file=${1:?path to the commit message is missing}
msg=$(cat "$msg_file")

# Strip comment lines and the diff `git commit -v` appends.
body=$(printf '%s\n' "$msg" | sed -e '/^#/d' -e '/^diff --git /,$d')
subject=$(printf '%s\n' "$body" | sed -e '/^[[:space:]]*$/d' -e 1q)

fail() {
    printf '\033[31mCommit refused:\033[0m %s\n' "$1" >&2
    shift
    for line in "$@"; do printf '  %s\n' "$line" >&2; done
    exit 1
}

# Merge, fixup and squash commits are not checked. Their form is dictated by
# git and gets resolved during the rebase anyway.
case "$subject" in
    "Merge "*|"Revert \""*|"fixup!"*|"squash!"*|"amend!"*) exit 0 ;;
esac

# --- Conventional Commits -------------------------------------------------
types='feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert'
if ! printf '%s' "$subject" |
    grep -Eq "^($types)(\([a-z0-9._/-]+\))?!?: .+"; then
    fail "The subject line does not follow Conventional Commits." \
        "Is:     $subject" \
        "Wanted: <type>[(scope)][!]: <description>" \
        "Types:  feat fix docs style refactor perf test build ci chore revert" \
        "" \
        "On a squash merge the subject determines the next version." \
        "Mark a breaking change with '!' after the type and 'BREAKING CHANGE:' in the body."
fi

if [ "${#subject}" -gt 100 ]; then
    fail "The subject line is ${#subject} characters long, 100 are allowed."
fi

# --- Attribution trailers -------------------------------------------------
# Claude Code appends a Co-Authored-By trailer and a "Generated with" line by
# default. The setting attribution.commit = "" is meant to turn that off but
# does not take effect reliably. Hence the hard block here.
if printf '%s\n' "$body" |
    grep -Eiq '^[[:space:]]*co-authored-by:.*(claude|anthropic|noreply@anthropic\.com)'; then
    fail "The message carries an AI attribution trailer." \
        "Remove the Co-Authored-By line." \
        "" \
        "Turn it off for good in ~/.claude/settings.json:" \
        '  { "attribution": { "commit": "", "pr": "" } }'
fi

if printf '%s\n' "$body" | grep -Eiq 'generated with .*claude code|🤖 generated with'; then
    fail "The message carries an AI generation line." \
        "Remove it. See the attribution setting in ~/.claude/settings.json."
fi

exit 0
