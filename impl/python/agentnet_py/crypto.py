import hashlib
from nacl.signing import VerifyKey


class CryptoError(ValueError):
    pass


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def verify_ed25519_hash(public_key: bytes, message_hash: bytes, signature: bytes) -> None:
    if len(public_key) != 32 or len(signature) != 64 or len(message_hash) != 32:
        raise CryptoError("invalid signature")
    try:
        vk = VerifyKey(public_key)
        vk.verify(message_hash, signature)
    except Exception as exc:
        raise CryptoError("invalid signature") from exc
