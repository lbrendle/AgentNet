export class CborError extends Error {
    constructor(message) {
        super(message);
    }
}
export function decodeCanonical(data) {
    const [value, pos] = decodeValue(data, 0);
    if (pos !== data.length) {
        throw new CborError("trailing bytes after cbor value");
    }
    return value;
}
export function encodeCanonical(value) {
    const out = [];
    encodeValue(value, out);
    return Uint8Array.from(out);
}
function decodeValue(data, pos) {
    if (pos >= data.length) {
        throw new CborError("unexpected end of input");
    }
    const initial = data[pos];
    pos += 1;
    const major = initial >> 5;
    const addl = initial & 0x1f;
    if (major === 0) {
        const [n, next] = readLen(data, pos, addl);
        return [n, next];
    }
    if (major === 1) {
        const [n, next] = readLen(data, pos, addl);
        if (n > BigInt(Number.MAX_SAFE_INTEGER)) {
            throw new CborError("integer overflow");
        }
        const val = -1n - n;
        return [val, next];
    }
    if (major === 2) {
        const [len, next] = readLen(data, pos, addl);
        const length = toNumber(len);
        const end = next + length;
        if (end > data.length) {
            throw new CborError("unexpected end of input");
        }
        return [data.slice(next, end), end];
    }
    if (major === 3) {
        const [len, next] = readLen(data, pos, addl);
        const length = toNumber(len);
        const end = next + length;
        if (end > data.length) {
            throw new CborError("unexpected end of input");
        }
        const text = new TextDecoder("utf-8", { fatal: true }).decode(data.slice(next, end));
        return [text, end];
    }
    if (major === 4) {
        const [len, next] = readLen(data, pos, addl);
        const length = toNumber(len);
        const items = [];
        let cursor = next;
        for (let i = 0; i < length; i += 1) {
            const [item, newPos] = decodeValue(data, cursor);
            items.push(item);
            cursor = newPos;
        }
        return [items, cursor];
    }
    if (major === 5) {
        const [len, next] = readLen(data, pos, addl);
        const length = toNumber(len);
        const entries = [];
        let cursor = next;
        for (let i = 0; i < length; i += 1) {
            const [key, pos1] = decodeValue(data, cursor);
            const [val, pos2] = decodeValue(data, pos1);
            entries.push([key, val]);
            cursor = pos2;
        }
        return [{ entries }, cursor];
    }
    if (major === 6) {
        throw new CborError("tags not supported");
    }
    if (major === 7) {
        if (addl === 20)
            return [false, pos];
        if (addl === 21)
            return [true, pos];
        if (addl === 22)
            return [null, pos];
        throw new CborError("unsupported simple value");
    }
    throw new CborError("unsupported major type");
}
function readLen(data, pos, addl) {
    if (addl <= 23)
        return [BigInt(addl), pos];
    if (addl === 24) {
        if (pos + 1 > data.length)
            throw new CborError("unexpected end of input");
        const val = BigInt(data[pos]);
        if (val < 24n)
            throw new CborError("non-canonical length");
        return [val, pos + 1];
    }
    if (addl === 25) {
        if (pos + 2 > data.length)
            throw new CborError("unexpected end of input");
        const val = BigInt((data[pos] << 8) | data[pos + 1]);
        if (val < 256n)
            throw new CborError("non-canonical length");
        return [val, pos + 2];
    }
    if (addl === 26) {
        if (pos + 4 > data.length)
            throw new CborError("unexpected end of input");
        const val = BigInt((data[pos] * 0x1000000) +
            (data[pos + 1] << 16) +
            (data[pos + 2] << 8) +
            data[pos + 3]);
        if (val < 65536n)
            throw new CborError("non-canonical length");
        return [val, pos + 4];
    }
    if (addl === 27) {
        if (pos + 8 > data.length)
            throw new CborError("unexpected end of input");
        const val = (BigInt(data[pos]) << 56n) |
            (BigInt(data[pos + 1]) << 48n) |
            (BigInt(data[pos + 2]) << 40n) |
            (BigInt(data[pos + 3]) << 32n) |
            (BigInt(data[pos + 4]) << 24n) |
            (BigInt(data[pos + 5]) << 16n) |
            (BigInt(data[pos + 6]) << 8n) |
            BigInt(data[pos + 7]);
        if (val < 4294967296n)
            throw new CborError("non-canonical length");
        return [val, pos + 8];
    }
    if (addl === 31)
        throw new CborError("indefinite length not allowed");
    throw new CborError("unsupported additional info");
}
function encodeValue(value, out) {
    if (typeof value === "boolean") {
        out.push(value ? 0xf5 : 0xf4);
        return;
    }
    if (value === null) {
        out.push(0xf6);
        return;
    }
    if (typeof value === "number") {
        if (!Number.isInteger(value))
            throw new CborError("invalid integer");
        encodeInt(BigInt(value), out);
        return;
    }
    if (typeof value === "bigint") {
        encodeInt(value, out);
        return;
    }
    if (typeof value === "string") {
        const encoded = new TextEncoder().encode(value);
        encodeMajor(out, 3, BigInt(encoded.length));
        out.push(...encoded);
        return;
    }
    if (value instanceof Uint8Array) {
        encodeMajor(out, 2, BigInt(value.length));
        out.push(...value);
        return;
    }
    if (Array.isArray(value)) {
        encodeMajor(out, 4, BigInt(value.length));
        for (const item of value) {
            encodeValue(item, out);
        }
        return;
    }
    if (isCborMap(value)) {
        const prepared = value.entries.map(([key, val]) => {
            const keyBytes = encodeCanonical(key);
            return { keyBytes, key, val };
        });
        prepared.sort((a, b) => {
            if (a.keyBytes.length !== b.keyBytes.length)
                return a.keyBytes.length - b.keyBytes.length;
            return compareBytes(a.keyBytes, b.keyBytes);
        });
        for (let i = 1; i < prepared.length; i += 1) {
            if (compareBytes(prepared[i - 1].keyBytes, prepared[i].keyBytes) === 0) {
                throw new CborError("duplicate map key");
            }
        }
        encodeMajor(out, 5, BigInt(prepared.length));
        for (const entry of prepared) {
            out.push(...entry.keyBytes);
            encodeValue(entry.val, out);
        }
        return;
    }
    throw new CborError("unsupported type");
}
function encodeInt(value, out) {
    if (value >= 0n) {
        encodeMajor(out, 0, value);
        return;
    }
    const n = -1n - value;
    if (n < 0n || n > 18446744073709551615n) {
        throw new CborError("integer overflow");
    }
    encodeMajor(out, 1, n);
}
function encodeMajor(out, major, value) {
    if (value < 0n)
        throw new CborError("invalid length");
    if (value <= 23n) {
        out.push((major << 5) | Number(value));
        return;
    }
    if (value <= 0xffn) {
        out.push((major << 5) | 24);
        out.push(Number(value));
        return;
    }
    if (value <= 0xffffn) {
        out.push((major << 5) | 25);
        out.push(Number((value >> 8n) & 0xffn));
        out.push(Number(value & 0xffn));
        return;
    }
    if (value <= 0xffffffffn) {
        out.push((major << 5) | 26);
        out.push(Number((value >> 24n) & 0xffn));
        out.push(Number((value >> 16n) & 0xffn));
        out.push(Number((value >> 8n) & 0xffn));
        out.push(Number(value & 0xffn));
        return;
    }
    out.push((major << 5) | 27);
    out.push(Number((value >> 56n) & 0xffn));
    out.push(Number((value >> 48n) & 0xffn));
    out.push(Number((value >> 40n) & 0xffn));
    out.push(Number((value >> 32n) & 0xffn));
    out.push(Number((value >> 24n) & 0xffn));
    out.push(Number((value >> 16n) & 0xffn));
    out.push(Number((value >> 8n) & 0xffn));
    out.push(Number(value & 0xffn));
}
function compareBytes(a, b) {
    const len = Math.min(a.length, b.length);
    for (let i = 0; i < len; i += 1) {
        if (a[i] !== b[i])
            return a[i] - b[i];
    }
    return a.length - b.length;
}
function toNumber(value) {
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
        throw new CborError("integer overflow");
    }
    return Number(value);
}
function isCborMap(value) {
    return typeof value === "object" && value !== null && "entries" in value;
}
