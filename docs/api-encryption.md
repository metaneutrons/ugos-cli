# Request encryption

Most UGOS endpoints accept plain requests. Some do not: the file manager's
v2 API and `downloadCenter/download/addV2` answer with an empty body unless
the payload is encrypted. This is what the web UI does, reconstructed from
its bundle (`desktop/static/main-*.js`, class `RequestEncrypt`).

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

The UI keeps a `WHITE_LIST_FORM_ENCRYPT`, among them `verify/login`,
`verify/check`, `wizard/`, various export and download endpoints, and
notably `downloadCenter/download/add` — but **not** its successor `addV2`,
which is why that one rejects plain requests.

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

## Verified

Against a live NAS on 2026-08-18: `kvm/manager/ShowLocalVirtualList` returns
the same data encrypted as it does plain, and `v2/filemgr/getDirFileListV2`
— which never answers a plain request — returns a directory listing. A path
the user cannot reach answers `1301, Access not allowed`, an ordinary
application error, which confirms the transport is sound.
