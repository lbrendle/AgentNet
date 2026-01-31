from nacl.signing import SigningKey

from .crypto import sha256


class SignError(ValueError):
    pass


def sign_ed25519_hash(secret_key: bytes, message_hash: bytes) -> bytes:
    if len(secret_key) != 32 or len(message_hash) != 32:
        raise SignError("invalid signature inputs")
    sk = SigningKey(secret_key)
    signed = sk.sign(message_hash)
    return signed.signature
