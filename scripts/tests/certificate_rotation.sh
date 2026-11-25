#!/usr/bin/env bash
# Run from the repository root after cargo build -p ugos-cli.
set -euo pipefail
umask 077
root=$(mktemp -d)
server=
cleanup() {
    if [[ -n "$server" ]]; then
        kill "$server" 2>/dev/null || true
        wait "$server" 2>/dev/null || true
    fi
    rm -rf -- "$root"
}
trap cleanup EXIT

fixture() {
    openssl req -x509 -newkey rsa:2048 -nodes -days 2 -subj "/CN=$2" \
        -addext "subjectAltName=DNS:$2" -keyout "$root/$1.key" -out "$root/$1.pem" 2>"$root/openssl.log"
    openssl x509 -noout -enddate -in "$root/$1.pem" > "$root/$1.expiry"
}
fixture old nas.example
fixture new nas.example
fixture standby standby.example

export HOME="$root" XDG_CONFIG_HOME="$root/.config" UGOS_HOST=127.0.0.1 UGOS_USER=fixture
UGOS_PASSWORD=$(openssl rand -hex 24)
export UGOS_PASSWORD
cfg="$XDG_CONFIG_HOME/ugos-cli"
if [[ $(uname -s) == Darwin ]]; then cfg="$HOME/Library/Application Support/ugos-cli"; fi
mkdir -p "$cfg"
touch "$root/writes" "$root/reject"
python3 scripts/tests/certificate_rotation.py "$root" "$cfg" &
server=$!
for ((i=0; i<100; i++)); do
    [[ -f "$root/port" ]] && break
    kill -0 "$server"
    sleep 0.1
done
UGOS_PORT=$(cat "$root/port")
export UGOS_PORT

invoke() {
    local expected=$1 status=0
    shift
    ./target/debug/ugos-cli certificate "$@" > "$root/output" 2>&1 || status=$?
    if grep -q 'PRIVATE KEY' "$root/output"; then echo 'Private key in output' >&2; exit 1; fi
    if [[ "$expected" == ok && "$status" != 0 ]] || [[ "$expected" == fail && "$status" == 0 ]]; then
        cat "$root/output" >&2
        exit 1
    fi
}
writes() {
    diff -u <(for event in "$@"; do echo "$event"; done) "$root/writes"
    : > "$root/writes"
}
pin() {
    jq -e --slurp '.[0] == .[1]' "$root/$1.pin" "$cfg/known_hosts.json" >/dev/null
}

rotate=(rotate --cert "$root/new.pem" --key "$root/new.key" --fallback-cert "$root/standby.pem")
echo '{}' > "$cfg/known_hosts.json"
invoke fail "${rotate[@]}"
jq -e 'length == 0' "$cfg/known_hosts.json" >/dev/null
cp "$root/old.pin" "$cfg/known_hosts.json"
touch "$root/hold-lock"
invoke fail "${rotate[@]}"
rm "$root/hold-lock"
invoke fail rotate --cert "$root/new.pem" --key "$root/old.key" --fallback-cert "$root/standby.pem"
invoke fail rotate --cert "$root/new.pem" --key "$root/new.key" --fallback-cert "$root/old.pem"
touch "$root/shared"
invoke fail "${rotate[@]}"
rm "$root/shared"
invoke fail import --replace-unused --cert "$root/new.pem" --key "$root/new.key"
writes

invoke fail "${rotate[@]}"
writes 'assign standby.example' 'delete nas.example' 'import new' 'import old'
pin standby
# Simulate an interruption between the reload and the pin update.
cp "$root/old.pin" "$cfg/known_hosts.json"
rm "$root/reject"
invoke ok "${rotate[@]}"
writes 'delete nas.example' 'import new' 'assign nas.example'
pin new
invoke ok "${rotate[@]}"
writes
echo 'Certificate rotation, verified TLS handovers, rollback and retry passed.'
