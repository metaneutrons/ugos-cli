#!/bin/sh
# Schnelle pre-commit-Prüfungen, die nicht kompilieren müssen.
# Alles, was länger als ein bis zwei Sekunden braucht, gehört in pre-push.
set -eu

max_bytes=${MAX_STAGED_BYTES:-5242880}   # 5 MiB
status=0

fail() { printf '\033[31m%s\033[0m %s\n' 'Commit abgelehnt:' "$1" >&2; status=1; }

# --- Direkter Commit auf den Standardbranch -------------------------------
# symbolic-ref statt rev-parse: funktioniert auch auf einem noch
# ungeborenen Branch, also im Repository ohne ersten Commit. rev-parse
# liefert dort nur "HEAD" und der Guard griffe nicht.
branch=$(git symbolic-ref --quiet --short HEAD 2>/dev/null || echo '')
default=$(git symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null |
    sed 's#^origin/##')
default=${default:-main}
if [ "$branch" = "$default" ] && [ "${ALLOW_COMMIT_ON_DEFAULT:-}" != '1' ]; then
    fail "Direkter Commit auf '$default'. Branch anlegen, oder ALLOW_COMMIT_ON_DEFAULT=1 setzen."
fi

# --- Große Dateien --------------------------------------------------------
# Fängt versehentlich hinzugefügte Archive, Images, DMGs und Schlüsseldateien,
# bevor sie unwiderruflich in der History stehen.
# `read -d` ist ein Bashismus und scheitert unter dash; deshalb zeilenweise.
# core.quotePath=false ist zwingend: sonst liefert git Nicht-ASCII-Pfade
# gequotet und escaped ("gr\303\266e.bin"), `[ -f ]` findet die Datei nicht
# und der Guard laesst sie stillschweigend durch. Praktisch verifiziert.
git -c core.quotePath=false diff --cached --name-only --diff-filter=AM | while IFS= read -r file; do
    [ -f "$file" ] || continue
    size=$(wc -c < "$file" | tr -d ' ')
    if [ "$size" -gt "$max_bytes" ]; then
        printf '\033[31mCommit abgelehnt:\033[0m %s ist %s Bytes (Grenze %s).\n' \
            "$file" "$size" "$max_bytes" >&2
        printf '  Gehört die Datei ins Repository? Sonst .gitignore ergänzen.\n' >&2
        printf '  Bewusste Ausnahme: MAX_STAGED_BYTES erhöhen.\n' >&2
        exit 1
    fi
done || status=1

# --- Verzeichnisse, die nie eingecheckt gehören ---------------------------
if git -c core.quotePath=false diff --cached --name-only |
    grep -Eq '(^|/)(node_modules|target|dist|build|\.next|coverage)/'; then
    fail "Ein Build- oder Abhängigkeitsverzeichnis ist gestaged. .gitignore prüfen."
fi

exit "$status"
