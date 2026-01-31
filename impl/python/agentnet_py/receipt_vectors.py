from pathlib import Path
import json

from .receipts import ReceiptLog


def main():
    vectors = json.loads(Path("spec/agentnet-test-vectors-v0.1.json").read_text())
    receipt_entry = next(v for v in vectors["vectors"] if v["id"] == "TV5_ReceiptChain")
    receipt1 = bytes.fromhex(receipt_entry["receipt1_payload_cbor_hex"])
    receipt2 = bytes.fromhex(receipt_entry["receipt2_payload_cbor_hex"])
    sig1 = bytes.fromhex(receipt_entry["receipt1_sig_hex"])
    sig2 = bytes.fromhex(receipt_entry["receipt2_sig_hex"])

    path = Path("/tmp/agentnet_receipts.log")
    if path.exists():
        path.unlink()
    log = ReceiptLog.open(path)
    log.append(receipt1, sig1)
    log.append(receipt2, sig2)

    log2 = ReceiptLog.open(path)
    if log2.last_seq() != 2:
        raise SystemExit("receipt log replay failed")

    print("receipt log verification complete")


if __name__ == "__main__":
    main()
