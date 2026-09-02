#!/bin/sh
# Prüft eine Commit-Nachricht gegen Conventional Commits und verbietet
# Attributions-Trailer von KI-Werkzeugen.
#
# Aufruf: check-commit-message.sh <pfad-zur-nachricht>
# Managerunabhängig. Funktioniert als commit-msg-Hook unter lefthook, husky,
# pre-commit oder direkt unter core.hooksPath.
set -eu

msg_file=${1:?Pfad zur Commit-Nachricht fehlt}
msg=$(cat "$msg_file")

# Kommentarzeilen und diff-Anhang von `git commit -v` entfernen.
body=$(printf '%s\n' "$msg" | sed -e '/^#/d' -e '/^diff --git /,$d')
subject=$(printf '%s\n' "$body" | sed -e '/^[[:space:]]*$/d' -e 1q)

fail() {
    printf '\033[31mCommit abgelehnt:\033[0m %s\n' "$1" >&2
    shift
    for line in "$@"; do printf '  %s\n' "$line" >&2; done
    exit 1
}

# Merge-, Fixup- und Squash-Commits werden nicht geprüft. Ihre Form ist von
# git vorgegeben und wird beim Rebase ohnehin aufgelöst.
case "$subject" in
    "Merge "*|"Revert \""*|"fixup!"*|"squash!"*|"amend!"*) exit 0 ;;
esac

# --- Conventional Commits -------------------------------------------------
types='feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert'
if ! printf '%s' "$subject" |
    grep -Eq "^($types)(\([a-z0-9._/-]+\))?!?: .+"; then
    fail "Die Betreffzeile folgt nicht Conventional Commits." \
        "Ist:   $subject" \
        "Soll:  <typ>[(scope)][!]: <beschreibung>" \
        "Typen: feat fix docs style refactor perf test build ci chore revert" \
        "" \
        "Der Betreff bestimmt bei Squash-Merge die nächste Version." \
        "Breaking Change über '!' hinter dem Typ und 'BREAKING CHANGE:' im Body."
fi

if [ "${#subject}" -gt 100 ]; then
    fail "Die Betreffzeile ist ${#subject} Zeichen lang, erlaubt sind 100."
fi

# --- Attributions-Trailer -------------------------------------------------
# Claude Code hängt standardmäßig einen Co-Authored-By-Trailer und eine
# "Generated with"-Zeile an. Das Setting attribution.commit = "" soll das
# abstellen, greift aber nicht zuverlässig. Deshalb hier hart blockieren.
if printf '%s\n' "$body" |
    grep -Eiq '^[[:space:]]*co-authored-by:.*(claude|anthropic|noreply@anthropic\.com)'; then
    fail "Die Nachricht enthält einen KI-Attributions-Trailer." \
        "Entferne die Co-Authored-By-Zeile." \
        "" \
        "Dauerhaft abstellen in ~/.claude/settings.json:" \
        '  { "attribution": { "commit": "", "pr": "" } }'
fi

if printf '%s\n' "$body" | grep -Eiq 'generated with .*claude code|🤖 generated with'; then
    fail "Die Nachricht enthält eine KI-Generierungszeile." \
        "Entferne sie. Siehe attribution-Setting in ~/.claude/settings.json."
fi

exit 0
