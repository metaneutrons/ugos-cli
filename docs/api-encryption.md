# Request encryption

UGOS can wrap request payloads in AES-GCM under an RSA-wrapped key. This is
what the web UI does, reconstructed from its bundle
(`desktop/static/main-*.js`, class `RequestEncrypt`).

**It is a substitute for TLS, not an addition to it.** The UI encrypts only
when the connection is *not* HTTPS — see [When the UI
encrypts](#when-the-ui-encrypts). Over HTTPS, which is all this client
speaks, the official UI sends plain requests too.

An earlier version of this document claimed that the file manager's v2 API
and `downloadCenter/download/addV2` reject plain requests. That was wrong on
both counts, and is corrected under [What actually
requires it](#what-actually-requires-it).

## The scheme

Per request:

1. Generate a key: a UUID with the dashes removed, so 32 hex characters. It
   is used as **32 raw ASCII bytes**, which makes this **AES-256**-GCM even
   though the UI calls its helper class `Aes128gcm`.
2. `X-Ugreen-Security-Code` = base64(RSA-PKCS#1-v1.5(public key, those 32
   characters)). With a 2048-bit key that is 344 base64 characters.
3. `X-Ugreen-Token` = base64(RSA(session token)) — the token is **not** sent
   in the clear, because the query string that normally carries it is now
   encrypted. Also 344 characters.
4. `X-Ugreen-Security-Key` = MD5(session token), hex.
5. `encrypt_query` = AES(qs.stringify(params)). The `token` key stays in that
   string with an empty value.
6. For a body: `{"encrypt_req_body": AES(json), "req_body_sha256":
   SHA256(json) as hex}`.
7. Responses arrive as `{"encrypt_resp_body": ...}` and decrypt with the same
   key.

AES payload layout: `base64(iv[12] || ciphertext || tag[16])`.

## Which endpoints skip it

The UI keeps a `WHITE_LIST_FORM_ENCRYPT` in its desktop bundle
(`/desktop/assets/main-*.js`), 77 paths long. The entries this project
touches are

| Path | Used by |
| --- | --- |
| `ugreen/v1/verify/login` | `ugos login` |
| `ugreen/v1/verify/check` | session check |
| `ugreen/v1/filemgr/fileUpload` | `ugos fs put` |
| `ugreen/v1/filemgr/downloadFile` | `ugos fs get` |
| `ugreen/v1/downloadCenter/download/add` | `ugos download add` |
| `ugreen/v1/log/query` | `ugos log list` |
| `ugreen/v1/log/export` | not implemented |
| `ugreen/v1/kvm/image/UploadUpk` | `ugos image upload` |
| `ugreen/v1/kvm/logs/ExportLogs` | not implemented |

The list explains behaviour that was found empirically first: every one of
these answers a plain request, while `downloadCenter/download/addV2` — absent
from the list — rejects one.

Two caveats. The list names `filemgr/fileUpload`, not the `fileUploadV2` that
`fs put` actually calls, yet `fileUploadV2` accepts plain multipart requests
all the same. Multipart bodies are plausibly exempt as a class rather than by
path, but that is inference from two observations, not something the bundle
states. And the constant governs *form* encryption only; whether the same list
also gates response encryption is untested.

## Two details that cost the most time

**The session token belongs inside the encrypted query**, with its real
value, *and* RSA-sealed in `X-Ugreen-Token`, *and* as an MD5 in
`X-Ugreen-Security-Key`. Sending it in only one or two of those three places
yields `1010, Token cannot be empty!` — the same message whether the token is
missing or the server simply cannot decrypt the query, which makes the error
misleading.

**An encrypted response has no envelope.** It arrives as a bare
`{"encrypt_resp_body": "..."}`; the usual `code`/`msg`/`data` structure is
*inside* the ciphertext. Decoding it as the normal envelope fails with
"missing field `code`" — which reads like a protocol error but actually means
the decryption step was skipped.

## When the UI encrypts

The decision sits in one condition in the desktop bundle:

```js
const s = e.baseURL?.startsWith?.("https") || false;   // is the connection HTTPS?
const c = this.whiteContentType.includes(contentType); // exempt content type?
const u = !WHITE_LIST_FORM_ENCRYPT.find(t => e.url.indexOf(t) !== -1);

if (!c && u && !s) {          // note the !s
    // encrypt: params -> encrypt_query, body -> encrypt_req_body
}
```

Encryption happens only when **`!s`**, that is when the base URL does not
start with `https`. UGOS is reachable over plain HTTP on port 9999 as well
as over HTTPS on 9443, and the wrapping exists to protect the former. On an
HTTPS connection the UI skips it entirely.

That explains the whole design. It also means a client using HTTPS behaves
exactly like the official UI by *not* encrypting.

## What actually requires it

Nothing that has been found. Measured against a live NAS on 2026-08-18,
across roughly twenty endpoints spanning KVM, Docker, sysinfo, taskmgr,
user, network, time and the file manager's v2 API: every one returned the
same result encrypted as in the clear, errors included. That covers reads,
writes (`v2/filemgr/rename`, `v2/filemgr/delPaths`,
`downloadCenter/download/addV2`, `docker/container/CreateContainer`) and a
12 KB payload.

Two earlier claims in this document were wrong:

- **`v2/filemgr/getDirFileListV2`** was said never to answer a plain
  request. It does. The original failure predates the commit that let a path
  choose its API version, so the request was going to a `v1` URL and failing
  for that reason.
- **`downloadCenter/download/addV2`** was said to reject plain requests. It
  does not; it answers `1302, Path does not exist` identically either way.
  The original failure was a payload-shape problem.

The lesson generalises: an endpoint failing plain and succeeding encrypted
is not evidence that it demands encryption, because the encrypted path was
usually fixed at the same time as something else.

## Should the client encrypt everything?

No, on the current evidence. The wrapping adds nothing over HTTPS, which is
the only transport this client uses, and the UI itself does not do it there.
Certificate pinning is what protects this connection — see
[api-tls.md](api-tls.md).

The scenario worth guarding against is UGOS one day requiring encryption
regardless of transport. That would mean breaking its own HTTPS clients, so
it is unlikely; and if it happened the change would be cheap, because the
implementation already exists and every endpoint tested accepts it.

Three things could never be wrapped in any case:

- `verify/check` hands out the RSA key the scheme needs.
- `verify/login` has no session token yet, which the scheme needs in three
  places at once. The password there is separately RSA-encrypted.
- Multipart uploads and binary downloads carry byte streams. The UI exempts
  these by content type, independently of the URL whitelist.

## Verified

Against a live NAS on 2026-08-18: `kvm/manager/ShowLocalVirtualList` returns
the same data encrypted as it does plain, and `v2/filemgr/getDirFileListV2`
returns a directory listing either way. A path
the user cannot reach answers `1301, Access not allowed`, an ordinary
application error, which confirms the transport is sound.
