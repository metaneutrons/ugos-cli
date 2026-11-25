"""Exercise the built CLI against a local HTTPS server that actually switches keys.

Run with python3 scripts/tests/certificate_rotation.py target/debug/ugos-cli.
Requires Python 3 and OpenSSL on Linux or macOS.
"""

import email.parser
import email.policy
import fcntl
import hashlib
import http.server
import io
import json
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import threading
import time
from urllib.parse import parse_qs, urlsplit
import zipfile


class Certificate:
    def __init__(self, root, label, subject):
        cert, key = root / (label + ".pem"), root / (label + ".key")
        subprocess.run([
            "openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
            "-subj", "/CN=" + subject, "-keyout", key, "-out", cert,
            "-addext", "subjectAltName=DNS:" + subject,
        ], check=True, capture_output=True)
        self.path, self.key_path, self.subject = cert, key, subject
        self.pem, self.key = cert.read_bytes(), key.read_bytes()
        self.pin = hashlib.sha256(ssl.PEM_cert_to_DER_cert(self.pem.decode())).hexdigest()
        expiry = subprocess.check_output(["openssl", "x509", "-in", cert, "-enddate", "-noout"], text=True)
        self.expiry = int(ssl.cert_time_to_seconds(expiry.strip().split("=", 1)[1]))
        self.tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        self.tls.load_cert_chain(cert, key)

    def archive(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w", compression=zipfile.ZIP_DEFLATED) as zipped:
            zipped.writestr("cert.pem", self.pem)
            zipped.writestr("privkey.pem", self.key)
        return data.getvalue()


class Nas(http.server.ThreadingHTTPServer):
    def get_request(self):
        sock, address = super().get_request()
        if self.reload_delay:
            self.reload_delay -= 1
        else:
            self.served = self.inventory[self.assignments["webserver"]]
        return self.served.tls.wrap_socket(sock, server_side=True), address


class Api(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def respond(self, data, code=200):
        self.data("application/json", json.dumps({"code": code, "msg": "test", "data": data}).encode())

    def data(self, content_type, body):
        self.send_response(200)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        nas = self.server
        path = urlsplit(self.path)
        if path.path == "/":
            return self.respond({})
        if path.path.endswith("/list"):
            rows = [
                dict(subject=s, issuer=c.subject, expired=c.expiry, isDefault=0,
                     beUsed=[k for k, v in nas.assignments.items() if v == s])
                for s, c in nas.inventory.items()
            ]
            self.respond({"result": rows})
        elif path.path.endswith("/getsettings"):
            self.respond({k: [v] for k, v in nas.assignments.items()})
        elif path.path.endswith("/export"):
            cert = nas.inventory[parse_qs(path.query)["cert"][0]]
            self.data("application/octet-stream", cert.archive())
        else:
            self.send_error(404)

    def do_POST(self):
        nas = self.server
        body = self.rfile.read(int(self.headers["Content-Length"]))
        if urlsplit(self.path).path.endswith("/settings"):
            settings = json.loads(body)
            assert settings.keys() == nas.assignments.keys()
            assert all(settings[k] == v for k, v in nas.assignments.items() if k != "webserver")
            assert settings["webserver"] in nas.inventory
            nas.writes.append("assign " + settings["webserver"])
            nas.reload_delay = 2
            nas.assignments = settings
            return self.respond({})
        assert urlsplit(self.path).path.endswith("/import")
        content_type = self.headers["Content-Type"].encode()
        message = email.parser.BytesParser(policy=email.policy.default).parsebytes(
            b"Content-Type: " + content_type + b"\r\n\r\n" + body)
        parts = list(message.iter_parts())
        assert [p.get_param("name", header="content-disposition") for p in parts] == ["file", "file"]
        key, cert = [p.get_payload(decode=True) for p in parts]
        bundle = next(c for c in nas.samples if c.pem.strip() == cert.strip() and c.key.strip() == key.strip())
        nas.writes.append("import " + bundle.pin)
        if nas.reject == bundle.pin or bundle.subject in nas.inventory:
            return self.respond({}, 8306)
        nas.inventory[bundle.subject] = bundle
        self.respond({})

    def do_DELETE(self):
        nas = self.server
        url = urlsplit(self.path)
        assert url.path.endswith("/cert/delete")
        subject = parse_qs(url.query)["cert"][0]
        assert subject not in nas.assignments.values() and nas.served.subject != subject
        del nas.inventory[subject]
        nas.writes.append("delete " + subject)
        self.respond({})


def run(binary, root):
    target = "nas.example"
    old = Certificate(root, "old", target)
    new = Certificate(root, "new", target)
    fallback = Certificate(root, "standby", "standby.example")
    with Nas(("127.0.0.1", 0), Api) as nas:
        nas.assignments = dict(webserver=target, ftps=fallback.subject, webdav=fallback.subject)
        nas.inventory = {target: old, fallback.subject: fallback}
        nas.served = old
        nas.reload_delay = 0
        nas.samples = [old, new, fallback]
        nas.writes = []
        nas.reject = new.pin
        threading.Thread(target=nas.serve_forever, daemon=True).start()
        port = nas.server_port
        cfg_root = root / ("Library/Application Support" if sys.platform == "darwin" else ".config")
        cfg = cfg_root / "ugos-cli"
        cfg.mkdir(parents=True)
        (cfg / "session.json").write_text(json.dumps(dict(
            user="test", token="test-session", host="127.0.0.1", port=port, created_at=int(time.time()))))
        pins = cfg / "known_hosts.json"
        address = "127.0.0.1:" + str(port)
        pins.write_text(json.dumps({address: old.pin}))
        env = dict(HOME=str(root), XDG_CONFIG_HOME=str(cfg_root), PATH=os.defpath,
                   UGOS_HOST="127.0.0.1", UGOS_PORT=str(port), UGOS_USER="test", UGOS_PASSWORD="test")

        def invoke(*args, success=True):
            result = subprocess.run([binary, "certificate", *args], env=env, capture_output=True, timeout=50)
            assert (result.returncode == 0) == success, result.stderr.decode()
            assert b"PRIVATE KEY" not in result.stdout + result.stderr

        args = ["rotate", "--cert", new.path, "--key", new.key_path, "--fallback-cert", fallback.path]
        pins.write_text("{}")
        invoke(*args, success=False)
        assert json.loads(pins.read_text()) == {}
        pins.write_text(json.dumps({address: old.pin}))
        lock = cfg / ("certificate-" + hashlib.sha256(address.encode()).hexdigest() + ".lock")
        with lock.open("a") as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            invoke(*args, success=False)
        invoke(*args[:-1], old.path, success=False)
        invoke("rotate", "--cert", new.path, "--key", old.key_path,
               "--fallback-cert", fallback.path, success=False)
        nas.assignments["ftps"] = target
        invoke(*args, success=False)
        nas.assignments["ftps"] = fallback.subject
        invoke("import", "--replace-unused", "--cert", new.path, "--key", new.key_path, success=False)
        assert not nas.writes
        invoke(*args, success=False)
        assert nas.writes == ["assign " + fallback.subject, "delete " + target, "import " + new.pin, "import " + old.pin]
        assert nas.inventory[target] is old and nas.assignments["webserver"] == fallback.subject
        assert json.loads(pins.read_text())[address] == fallback.pin

        # Resume with a stale pin, as after interruption immediately after a reload.
        pins.write_text(json.dumps({address: old.pin}))
        nas.reject = None
        nas.writes.clear()
        invoke(*args)
        assert nas.writes == ["delete " + target, "import " + new.pin, "assign " + target]
        assert nas.inventory[target] is new and json.loads(pins.read_text())[address] == new.pin
        nas.writes.clear()
        invoke(*args)
        assert not nas.writes
        nas.shutdown()


if __name__ == "__main__":
    with tempfile.TemporaryDirectory(prefix="ugos-cert-test-") as temp:
        run(Path(sys.argv[1]).resolve(), Path(temp))
    print("Certificate rotation, verified TLS handovers, rollback and retry passed.")
