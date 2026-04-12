# UGOS API Authentication

## Overview

UGOS (UGREEN NAS OS) uses a multi-step authentication flow combining RSA encryption and session tokens.

## Auth Flow

### Step 1: Obtain RSA Public Key

```
POST /ugreen/v1/verify/check
Content-Type: application/json

{"username": "<username>"}
```

**Response**: HTTP 200 with `x-rsa-token` header containing a base64-encoded PEM RSA public key.

### Step 2: Encrypt Password

Encrypt the plaintext password using the RSA public key with **PKCS1v1.5** padding. Base64-encode the ciphertext.

### Step 3: Login

```
POST /ugreen/v1/verify/login
Content-Type: application/json

{
  "username": "<username>",
  "password": "<base64-rsa-encrypted-password>",
  "keepalive": true,
  "otp": false
}
```

**Response**:
```json
{
  "code": 200,
  "msg": "success",
  "data": {
    "auth_type": "header",
    "token": "<session-token>",
    "static_token": "<static-token>",
    "uid": 1000,
    "username": "fabian",
    "role": "admin",
    "model": "DXP480T Plus",
    "nas_name": "picard",
    "system_version": "1.14.1.0107",
    "public_key": "<base64-pem>",
    "http_port": 9999,
    "https_port": 9443,
    ...
  }
}
```

**Set-Cookie**: `token=<url-encoded-token>; Path=/; HttpOnly; SameSite=Strict`

### Step 4: Authenticated Requests

All subsequent API calls require **both**:
1. The session cookie (set automatically by cookie jar)
2. The token as a URL query parameter: `?token=<token>`

The `auth_type` field in the login response indicates the token delivery method. Despite `auth_type: "header"`, the KVM app requires `?token=` as a URL parameter (combined with cookies).

## Reference Implementation (Python)

This working code was used to validate the auth flow against picard (DXP480T Plus, UGOS 1.14.1.0107):

```python
import urllib.request, ssl, json, base64, subprocess, http.cookiejar

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE
BASE = "https://192.168.2.5:9443/ugreen/v1"

cj = http.cookiejar.CookieJar()
opener = urllib.request.build_opener(
    urllib.request.HTTPCookieProcessor(cj),
    urllib.request.HTTPSHandler(context=ctx))

# Step 1: POST /verify/check with username → get RSA public key from x-rsa-token header
resp = opener.open(urllib.request.Request(
    f"{BASE}/verify/check",
    data=json.dumps({"username": "fabian"}).encode(),
    headers={"Content-Type": "application/json"}))
rsa_b64 = resp.headers.get("x-rsa-token")       # base64-encoded PEM
pem = base64.b64decode(rsa_b64).decode()          # "-----BEGIN RSA PUBLIC KEY-----\n..."

# Step 2: RSA-encrypt password with PKCS1v1.5 padding
with open("/tmp/ugos_pub.pem", "w") as f:
    f.write(pem)
ciphertext = subprocess.check_output([
    "openssl", "pkeyutl", "-encrypt",
    "-pubin", "-inkey", "/tmp/ugos_pub.pem",
    "-pkeyopt", "rsa_padding_mode:pkcs1"           # PKCS1v1.5, NOT OAEP
], input=b"the-password")
encrypted_password = base64.b64encode(ciphertext).decode()

# Step 3: POST /verify/login with encrypted password → get token + cookies
resp2 = opener.open(urllib.request.Request(
    f"{BASE}/verify/login",
    data=json.dumps({
        "username": "fabian",
        "password": encrypted_password,
        "keepalive": True,
        "otp": False
    }).encode(),
    headers={"Content-Type": "application/json"}))
login = json.loads(resp2.read())
token = login["data"]["token"]                     # e.g. "E430830B3EDA4A8C..."
# Cookies are now in the cookie jar (token=...; token_uid=...)

# Step 4: Authenticated API call — cookies + ?token= URL parameter
r = opener.open(f"{BASE}/kvm/manager/ShowLocalVirtualList?token={token}")
vms = json.loads(r.read())
# {"code": 200, "data": {"result": [...]}}
```

### Key Details

- **RSA padding**: PKCS1v1.5 (`rsa_padding_mode:pkcs1`). OAEP does NOT work.
- **PEM format**: The `x-rsa-token` header decodes to a standard PEM with `-----BEGIN RSA PUBLIC KEY-----`.
- **Token delivery**: Despite `auth_type: "header"` in the login response, the KVM app requires `?token=` as a URL query parameter. The `X-Ugreen-Token` header alone does NOT work.
- **Cookies required**: The `Set-Cookie` from login (`token=...; token_uid=...`) must be sent alongside the `?token=` parameter. Both are needed.
- **GET for mutations**: UGOS uses GET for destructive operations (PowerOn, Shutdown, Delete). Parameters go in the query string.

## Session Management

- Tokens expire after inactivity (exact timeout unknown, likely 30min)
- `keepalive: true` extends session lifetime
- Heartbeat: `GET /ugreen/v1/verify/heartbeat`
- Logout: `POST /ugreen/v1/verify/logout`

## API Base URL

```
https://<nas-ip>:9443/ugreen/v1/
```

Port 9443 is HTTPS, port 9999 is HTTP.

## Error Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 1003 | Incorrect account or password |
| 1005 | Parameter error |
| 1024 | Login expired / invalid token |
| 3004 | VM operation failed (e.g. shutdown on stopped VM) |
| 9404 | App not found (service not installed) |
| 9405 | App service error |

## Response Envelope

Every UGOS API response has this outer shape:

```json
{"code": 200, "msg": "success", "data": <T>, "time": 0.006}
```

The `data` field is polymorphic. Observed shapes:

| Shape | Example | Used by |
|-------|---------|---------|
| `{result: [...]}` | Array of items | All list endpoints |
| `{result: {...}}` | Single object | GetNetworkByName, ShowLocalVirtualMachine |
| `{result: "successful"}` | Bare string | PowerOn, Shutdown, Reboot, ForcedShutdown, ForcedRestart, Delete |
| `{result: false}` | Bare bool | CheckImageName |
| `{memoryStatus: 0}` | Ad-hoc fields (no `result` key) | CheckResource |
| `{}` | Empty object | Error responses, some edge cases |

### Rust modeling suggestion

```rust
/// Raw API envelope. Deserialize `data` as `serde_json::Value` first,
/// then parse into the expected type per-endpoint.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse<T> {
    code: i32,
    msg: String,
    data: T,
    time: f64,
}

/// For endpoints returning {result: T}
#[derive(Deserialize)]
struct ResultWrapper<T> {
    result: T,
}

// Usage:
// List:     ApiResponse<ResultWrapper<Vec<VmSummary>>>
// Detail:   ApiResponse<ResultWrapper<VmDetail>>  (ShowLocalVirtualMachine wraps in result too? — no, it doesn't)
// Action:   ApiResponse<ResultWrapper<String>>     where result == "successful"
// Check:    ApiResponse<ResultWrapper<bool>>
// Adhoc:    ApiResponse<CheckResourceData>         with custom struct
// Empty:    ApiResponse<serde_json::Value>         and ignore data
```

**Important**: `ShowLocalVirtualMachine` does NOT use `result` — its data is
the `VmDetail` directly under `data`. Most other endpoints use `{result: T}`.
Verify each endpoint individually.

