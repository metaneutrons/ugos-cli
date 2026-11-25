"""Loopback HTTPS fixture used by certificate_rotation.sh."""

import email.parser
import email.policy
import fcntl
import hashlib
import http.server
import io
import json
from pathlib import Path
import secrets
import ssl
import sys
import time
from urllib.parse import parse_qs, urlsplit
import zipfile


class Certificate:
    def __init__(self, root, label, subject):
        cert, key = root / (label + ".pem"), root / (label + ".key")
        self.label, self.subject = label, subject
        self.pem, self.key = cert.read_bytes(), key.read_bytes()
        self.pin = hashlib.sha256(ssl.PEM_cert_to_DER_cert(self.pem.decode())).hexdigest()
        expiry = (root / (label + ".expiry")).read_text()
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
        if (self.root / "hold-lock").exists():
            fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        else:
            fcntl.flock(self.lock, fcntl.LOCK_UN)
        if self.reload_delay:
            self.reload_delay -= 1
        else:
            self.served = self.inventory[self.assignments["webserver"]]
        return self.served.tls.wrap_socket(sock, server_side=True), address

    def record(self, operation):
        with (self.root / "writes").open("a") as journal:
            print(operation, file=journal)


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
                     beUsed=[k for k, v in nas.assignments.items() if v == s] +
                         (["ftps"] if s == "nas.example" and (nas.root / "shared").exists() else []))
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
            nas.record("assign " + settings["webserver"])
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
        nas.record("import " + bundle.label)
        if (bundle.label == "new" and (nas.root / "reject").exists()) or bundle.subject in nas.inventory:
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
        nas.record("delete " + subject)
        self.respond({})


def serve(root, cfg):
    target = "nas.example"
    old = Certificate(root, "old", target)
    new = Certificate(root, "new", target)
    standby = Certificate(root, "standby", "standby.example")
    with Nas(("127.0.0.1", 0), Api) as nas:
        nas.root = root
        nas.assignments = dict(webserver=target, ftps=standby.subject, webdav=standby.subject)
        nas.inventory = {target: old, standby.subject: standby}
        nas.served, nas.reload_delay = old, 0
        nas.samples = [old, new, standby]
        port = nas.server_port
        address = "127.0.0.1:" + str(port)
        (cfg / "session.json").write_text(json.dumps(dict(
            user="fixture", token=secrets.token_hex(24), host="127.0.0.1",
            port=port, created_at=int(time.time()))))
        for item in nas.samples:
            (root / (item.label + ".pin")).write_text(json.dumps({address: item.pin}))
        lock = cfg / ("certificate-" + hashlib.sha256(address.encode()).hexdigest() + ".lock")
        with lock.open("a") as nas.lock:
            ready = root / "port.tmp"
            ready.write_text(str(port))
            ready.rename(root / "port")
            nas.serve_forever()


if __name__ == "__main__":
    serve(Path(sys.argv[1]), Path(sys.argv[2]))
