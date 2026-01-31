from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Tuple

from .cbor import decode_canonical, CborMap, CborValue
from .crypto import sha256, verify_ed25519_hash


class ReceiptError(ValueError):
    pass


@dataclass
class ReceiptRecord:
    payload: bytes
    receipt_hash: bytes
    signature: bytes


class ReceiptLog:
    def __init__(self, path: Path, last_hash: bytes, last_seq: int):
        self._path = path
        self._file = path.open("ab+")
        self._last_hash = last_hash
        self._last_seq = last_seq

    @classmethod
    def open(cls, path: str | Path) -> "ReceiptLog":
        path = Path(path)
        if not path.exists():
            path.touch()
        log = cls(path, b"\x00" * 32, 0)
        log._replay()
        return log

    def append(self, payload: bytes, signature: bytes) -> ReceiptRecord:
        return self._append_internal(payload, signature, None)

    def append_verified(self, payload: bytes, signature: bytes, public_key: bytes) -> ReceiptRecord:
        return self._append_internal(payload, signature, public_key)

    def last_hash(self) -> bytes:
        return self._last_hash

    def last_seq(self) -> int:
        return self._last_seq

    def _append_internal(self, payload: bytes, signature: bytes, public_key: Optional[bytes]) -> ReceiptRecord:
        receipt = _parse_receipt_payload(decode_canonical(payload))
        if receipt["seq"] != self._last_seq + 1:
            raise ReceiptError("receipt sequence mismatch")
        if receipt["prev_hash"] != self._last_hash:
            raise ReceiptError("receipt prev_hash mismatch")
        receipt_hash = sha256(payload)
        if public_key is not None:
            verify_ed25519_hash(public_key, receipt_hash, signature)
        self._write_record(payload, signature)
        self._last_hash = receipt_hash
        self._last_seq = receipt["seq"]
        return ReceiptRecord(payload=payload, receipt_hash=receipt_hash, signature=signature)

    def _replay(self) -> None:
        self._file.seek(0)
        while True:
            len_bytes = self._file.read(4)
            if not len_bytes:
                break
            if len(len_bytes) != 4:
                raise ReceiptError("invalid receipt log length")
            payload_len = int.from_bytes(len_bytes, "big")
            payload = self._file.read(payload_len)
            if len(payload) != payload_len:
                raise ReceiptError("invalid receipt log payload")
            sig_len_bytes = self._file.read(4)
            if len(sig_len_bytes) != 4:
                raise ReceiptError("invalid receipt log signature length")
            sig_len = int.from_bytes(sig_len_bytes, "big")
            signature = self._file.read(sig_len)
            if len(signature) != sig_len:
                raise ReceiptError("invalid receipt log signature")
            receipt = _parse_receipt_payload(decode_canonical(payload))
            if receipt["seq"] != self._last_seq + 1:
                raise ReceiptError("receipt sequence mismatch")
            if receipt["prev_hash"] != self._last_hash:
                raise ReceiptError("receipt prev_hash mismatch")
            self._last_hash = sha256(payload)
            self._last_seq = receipt["seq"]
        self._file.seek(0, 2)

    def _write_record(self, payload: bytes, signature: bytes) -> None:
        self._file.write(len(payload).to_bytes(4, "big"))
        self._file.write(payload)
        self._file.write(len(signature).to_bytes(4, "big"))
        self._file.write(signature)
        self._file.flush()


def _parse_receipt_payload(value: CborValue) -> dict:
    if not isinstance(value, CborMap):
        raise ReceiptError("expected receipt payload map")
    entries = value.entries

    def get_required(key: int) -> CborValue:
        for k, v in entries:
            if isinstance(k, int) and k == key:
                return v
        raise ReceiptError("missing required key")

    receipt_id = get_required(0)
    ts = get_required(1)
    actor = get_required(2)
    pairing = get_required(3)
    community = get_required(4)
    event = get_required(5)
    auth = get_required(6)
    economics = get_required(7)
    prev_hash = get_required(8)
    seq = get_required(9)

    if not isinstance(receipt_id, str):
        raise ReceiptError("receipt_id must be text")
    if not isinstance(ts, int):
        raise ReceiptError("ts must be int")
    if not isinstance(actor, str):
        raise ReceiptError("actor must be text")
    if not (isinstance(pairing, str) or pairing is None):
        raise ReceiptError("pairing_id must be text or null")
    if not (isinstance(community, str) or community is None):
        raise ReceiptError("community_id must be text or null")
    if not isinstance(prev_hash, (bytes, bytearray)):
        raise ReceiptError("prev_hash must be bytes")
    if not isinstance(seq, int):
        raise ReceiptError("seq must be int")

    return {
        "receipt_id": receipt_id,
        "ts": ts,
        "actor": actor,
        "pairing_id": pairing,
        "community_id": community,
        "event": event,
        "auth": auth,
        "economics": economics,
        "prev_hash": bytes(prev_hash),
        "seq": seq,
    }
