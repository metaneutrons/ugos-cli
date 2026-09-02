#!/bin/sh
# gitleaks nur über die Commits, die tatsächlich gepusht werden.
# Ein voller History-Scan bei jedem Push wird mit wachsendem Repository
# unbrauchbar langsam und erzieht zu --no-verify.
set -eu

command -v gitleaks >/dev/null 2>&1 || {
    printf '\033[31mPush abgelehnt:\033[0m gitleaks fehlt (brew install gitleaks).\n' >&2
    exit 1
}

# Bereich bestimmen, von präzise nach grob.
range=''
if upstream=$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null); then
    range="${upstream}..HEAD"
elif head_ref=$(git symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null); then
    range="${head_ref}..HEAD"
fi

if [ -z "$range" ]; then
    # Kein Remote-Bezug, etwa beim ersten Push eines neuen Repositories.
    printf 'gitleaks: kein Remote-Bezug, vollständiger Scan.\n' >&2
    exec gitleaks git --redact --verbose .
fi

# Leerer Bereich heißt: nichts Neues zu prüfen.
if [ -z "$(git rev-list --max-count=1 "$range" 2>/dev/null || true)" ]; then
    printf 'gitleaks: keine neuen Commits in %s.\n' "$range" >&2
    exit 0
fi

printf 'gitleaks: prüfe %s\n' "$range" >&2
exec gitleaks git --redact --verbose --log-opts="$range" .
