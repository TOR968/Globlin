import base64
import hashlib
import os
import pathlib
import sys
import time

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

KEY_ID_LENGTH = 8
SEED_LENGTH = 32


def load_key(encoded):
    raw = base64.b64decode(encoded.strip())
    if len(raw) != KEY_ID_LENGTH + SEED_LENGTH:
        raise SystemExit("MINISIGN_SECRET_KEY must be base64 of an 8-byte key id and a 32-byte seed")
    return raw[:KEY_ID_LENGTH], Ed25519PrivateKey.from_private_bytes(raw[KEY_ID_LENGTH:])


def signature(path, key_id, key, timestamp):
    digest = hashlib.blake2b(path.read_bytes(), digest_size=64).digest()
    signed = key.sign(digest)
    trusted = f"timestamp:{timestamp}\tfile:{path.name}\thashed"
    global_signature = key.sign(signed + trusted.encode())
    return (
        "untrusted comment: signature from the Globlin release key\n"
        + base64.b64encode(b"ED" + key_id + signed).decode()
        + "\n"
        + f"trusted comment: {trusted}\n"
        + base64.b64encode(global_signature).decode()
        + "\n"
    )


def main(paths):
    encoded = os.environ.get("MINISIGN_SECRET_KEY", "")
    if not encoded.strip():
        raise SystemExit("MINISIGN_SECRET_KEY is not set; refusing to publish unsigned files")
    key_id, key = load_key(encoded)
    timestamp = int(time.time())
    for name in paths:
        path = pathlib.Path(name)
        target = path.with_name(path.name + ".minisig")
        target.write_text(signature(path, key_id, key, timestamp), encoding="ascii", newline="\n")
        print(f"signed {path.name}")


if __name__ == "__main__":
    main(sys.argv[1:])
