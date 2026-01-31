from __future__ import annotations

from dataclasses import dataclass
from typing import List, Tuple, Union


@dataclass(frozen=True)
class CborMap:
    entries: List[Tuple["CborValue", "CborValue"]]


CborValue = Union[int, bytes, str, List["CborValue"], CborMap, bool, None]


class CborError(ValueError):
    pass


def decode_canonical(data: bytes) -> CborValue:
    pos = 0
    value, pos = _decode_value(data, pos)
    if pos != len(data):
        raise CborError("trailing bytes after cbor value")
    return value


def encode_canonical(value: CborValue) -> bytes:
    out = bytearray()
    _encode_value(value, out)
    return bytes(out)


def _decode_value(data: bytes, pos: int) -> Tuple[CborValue, int]:
    if pos >= len(data):
        raise CborError("unexpected end of input")
    initial = data[pos]
    pos += 1
    major = initial >> 5
    addl = initial & 0x1F

    if major == 0:
        n, pos = _read_len(data, pos, addl)
        return n, pos
    if major == 1:
        n, pos = _read_len(data, pos, addl)
        if n > (2**63 - 1):
            raise CborError("integer overflow")
        return -1 - n, pos
    if major == 2:
        length, pos = _read_len(data, pos, addl)
        end = pos + length
        if end > len(data):
            raise CborError("unexpected end of input")
        return data[pos:end], end
    if major == 3:
        length, pos = _read_len(data, pos, addl)
        end = pos + length
        if end > len(data):
            raise CborError("unexpected end of input")
        try:
            text = data[pos:end].decode("utf-8")
        except UnicodeDecodeError as exc:
            raise CborError("invalid utf-8") from exc
        return text, end
    if major == 4:
        length, pos = _read_len(data, pos, addl)
        items: List[CborValue] = []
        for _ in range(length):
            item, pos = _decode_value(data, pos)
            items.append(item)
        return items, pos
    if major == 5:
        length, pos = _read_len(data, pos, addl)
        entries: List[Tuple[CborValue, CborValue]] = []
        for _ in range(length):
            key, pos = _decode_value(data, pos)
            val, pos = _decode_value(data, pos)
            entries.append((key, val))
        return CborMap(entries), pos
    if major == 6:
        raise CborError("tags not supported")
    if major == 7:
        if addl == 20:
            return False, pos
        if addl == 21:
            return True, pos
        if addl == 22:
            return None, pos
        raise CborError("unsupported simple value")

    raise CborError("unsupported major type")


def _read_len(data: bytes, pos: int, addl: int) -> Tuple[int, int]:
    if addl <= 23:
        return addl, pos
    if addl == 24:
        if pos + 1 > len(data):
            raise CborError("unexpected end of input")
        val = data[pos]
        pos += 1
        if val < 24:
            raise CborError("non-canonical length")
        return val, pos
    if addl == 25:
        if pos + 2 > len(data):
            raise CborError("unexpected end of input")
        val = int.from_bytes(data[pos:pos+2], "big")
        pos += 2
        if val < 256:
            raise CborError("non-canonical length")
        return val, pos
    if addl == 26:
        if pos + 4 > len(data):
            raise CborError("unexpected end of input")
        val = int.from_bytes(data[pos:pos+4], "big")
        pos += 4
        if val < 65536:
            raise CborError("non-canonical length")
        return val, pos
    if addl == 27:
        if pos + 8 > len(data):
            raise CborError("unexpected end of input")
        val = int.from_bytes(data[pos:pos+8], "big")
        pos += 8
        if val < 4294967296:
            raise CborError("non-canonical length")
        return val, pos
    if addl == 31:
        raise CborError("indefinite length not allowed")
    raise CborError("unsupported additional info")


def _encode_value(value: CborValue, out: bytearray) -> None:
    if isinstance(value, bool):
        out.append(0xF5 if value else 0xF4)
        return
    if value is None:
        out.append(0xF6)
        return
    if isinstance(value, int):
        if value >= 0:
            _encode_major(out, 0, value)
            return
        n = -1 - value
        if n < 0 or n > (2**64 - 1):
            raise CborError("integer overflow")
        _encode_major(out, 1, n)
        return
    if isinstance(value, bytes):
        _encode_major(out, 2, len(value))
        out.extend(value)
        return
    if isinstance(value, str):
        encoded = value.encode("utf-8")
        _encode_major(out, 3, len(encoded))
        out.extend(encoded)
        return
    if isinstance(value, list):
        _encode_major(out, 4, len(value))
        for item in value:
            _encode_value(item, out)
        return
    if isinstance(value, CborMap):
        entries = []
        for key, val in value.entries:
            key_bytes = encode_canonical(key)
            entries.append((key_bytes, key, val))
        entries.sort(key=lambda x: (len(x[0]), x[0]))
        for i in range(1, len(entries)):
            if entries[i-1][0] == entries[i][0]:
                raise CborError("duplicate map key")
        _encode_major(out, 5, len(entries))
        for key_bytes, _key, val in entries:
            out.extend(key_bytes)
            _encode_value(val, out)
        return
    raise CborError("unsupported type")


def _encode_major(out: bytearray, major: int, value: int) -> None:
    if value < 0:
        raise CborError("invalid length")
    if value <= 23:
        out.append((major << 5) | value)
        return
    if value <= 0xFF:
        out.append((major << 5) | 24)
        out.append(value)
        return
    if value <= 0xFFFF:
        out.append((major << 5) | 25)
        out.extend(value.to_bytes(2, "big"))
        return
    if value <= 0xFFFF_FFFF:
        out.append((major << 5) | 26)
        out.extend(value.to_bytes(4, "big"))
        return
    out.append((major << 5) | 27)
    out.extend(value.to_bytes(8, "big"))
