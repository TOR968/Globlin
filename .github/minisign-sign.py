import base64
import hashlib
import os
import pathlib
import sys
import time

KEY_ID_LENGTH = 8
SEED_LENGTH = 32

FIELD = 2**255 - 19
ORDER = 2**252 + 27742317777372353535851937790883648493
CURVE_D = -121665 * pow(121666, -1, FIELD) % FIELD
BASE_X = 15112221349535400772501151409588531511454012693041857206046113283949847762202
BASE_Y = 46316835694926478169428394003475163141307993866256225615783033603165251855960
BASE = (BASE_X, BASE_Y, 1, BASE_X * BASE_Y % FIELD)
IDENTITY = (0, 1, 1, 0)

KNOWN_SEED = bytes.fromhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
KNOWN_SIGNATURE = bytes.fromhex(
    "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
)


def add(left, right):
    x1, y1, z1, t1 = left
    x2, y2, z2, t2 = right
    a = (y1 - x1) * (y2 - x2) % FIELD
    b = (y1 + x1) * (y2 + x2) % FIELD
    c = 2 * t1 * t2 * CURVE_D % FIELD
    d = 2 * z1 * z2 % FIELD
    e, f, g, h = b - a, d - c, d + c, b + a
    return (e * f % FIELD, g * h % FIELD, f * g % FIELD, e * h % FIELD)


def multiply(scalar, point):
    result = IDENTITY
    while scalar:
        if scalar & 1:
            result = add(result, point)
        point = add(point, point)
        scalar >>= 1
    return result


def compress(point):
    x, y, z, _ = point
    inverse = pow(z, FIELD - 2, FIELD)
    x, y = x * inverse % FIELD, y * inverse % FIELD
    return (y | (x & 1) << 255).to_bytes(32, "little")


def reduced(data):
    return int.from_bytes(hashlib.sha512(data).digest(), "little") % ORDER


def sign(seed, message):
    expanded = hashlib.sha512(seed).digest()
    secret = int.from_bytes(expanded[:32], "little") & ((1 << 254) - 8) | (1 << 254)
    public = compress(multiply(secret, BASE))
    nonce = reduced(expanded[32:] + message)
    commitment = compress(multiply(nonce, BASE))
    challenge = reduced(commitment + public + message)
    return commitment + ((nonce + challenge * secret) % ORDER).to_bytes(32, "little")


def load_key(encoded):
    raw = base64.b64decode(encoded.strip())
    if len(raw) != KEY_ID_LENGTH + SEED_LENGTH:
        raise SystemExit("MINISIGN_SECRET_KEY must be base64 of an 8-byte key id and a 32-byte seed")
    return raw[:KEY_ID_LENGTH], raw[KEY_ID_LENGTH:]


def signature(path, key_id, seed, timestamp, version):
    digest = hashlib.blake2b(path.read_bytes(), digest_size=64).digest()
    signed = sign(seed, digest)
    trusted = f"timestamp:{timestamp}\tfile:{path.name}\tversion:{version}\thashed"
    global_signature = sign(seed, signed + trusted.encode())
    return (
        "untrusted comment: signature from the Globlin release key\n"
        + base64.b64encode(b"ED" + key_id + signed).decode()
        + "\n"
        + f"trusted comment: {trusted}\n"
        + base64.b64encode(global_signature).decode()
        + "\n"
    )


def main(version, paths):
    if sign(KNOWN_SEED, b"") != KNOWN_SIGNATURE:
        raise SystemExit("the Ed25519 signer fails the RFC 8032 test vector; refusing to sign")
    if not version or version.startswith("v") or not paths:
        raise SystemExit("usage: minisign-sign.py <version without v> <file>...")
    encoded = os.environ.get("MINISIGN_SECRET_KEY", "")
    if not encoded.strip():
        raise SystemExit("MINISIGN_SECRET_KEY is not set; refusing to publish unsigned files")
    key_id, seed = load_key(encoded)
    timestamp = int(time.time())
    for name in paths:
        path = pathlib.Path(name)
        target = path.with_name(path.name + ".minisig")
        target.write_text(signature(path, key_id, seed, timestamp, version), encoding="ascii", newline="\n")
        print(f"signed {path.name} for {version}")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "", sys.argv[2:])
