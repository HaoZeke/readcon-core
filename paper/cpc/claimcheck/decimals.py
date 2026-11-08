#!/usr/bin/env python3
"""Decimal reading and shortest printing of readcon-core on the parse-number-fxx test data and the
fast_float benchmark files.

The paper says the reader rounds each decimal to the nearest binary64 value, ties to even, and rejects a
decimal that rounds to an infinity, and that the default writer prints the shortest decimal that reads back
as the same value.  The data are

- the 14 files of Nigel Tao's parse-number-fxx-test-data at commit ``TAO_COMMIT``, 5,299,993 strings, each
  with the binary64 bits that a correct reader returns, collected from the test suites of
  double-conversion, Wuffs, IBM FPGen, fast_float, fast_double_parser, RapidJSON, Ryu, FreeType and the
  fptest generator;
- ``canada.txt`` and ``mesh.txt`` of Lemire's simple_fastfloat_benchmark at tag v0.1.0 (``LEMIRE_COMMIT``),
  the two real-data files of the fast_float paper, whose expected values are those of CPython's ``float``.

Each file is pinned by its size and its number of lines, and the record keeps the SHA-256 of what was read.
Every string with a finite expected value that is a CON decimal (CON-token.2) goes into one CON frame of a
file per data file, as the x coordinate of its own atom.  The frame is read twice, by ``read_positions`` from
the file and by ``read_con_string`` from its text, and every x is compared with the expected bits.  The
frames read by ``read_con_string`` are written again by ``write_con_string`` at its default precision, and
every x token printed is read back by CPython and compared, digit for digit, with the shortest form
``repr`` gives.  Every other string is read alone, in a one-atom frame: one whose expected value is an
infinity has to be rejected as a non-finite value, and one that is not a CON decimal as an invalid number
format (CON-num.1 and CON-num.8).

The record is ``data/claimcheck/decimals.json``.
"""

from __future__ import annotations

import re
import struct
import sys
import time
from decimal import Decimal
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import cc_common as cc

TAO_COMMIT = "55d79b184b7d8fac2e143e89dc19b766ec4e54b8"
TAO_URL = f"https://raw.githubusercontent.com/nigeltao/parse-number-fxx-test-data/{TAO_COMMIT}/data/"
TAO = {
    "exhaustive-float16.txt": (1409262, 31745),
    "freetype-2-7.txt": (128556, 3566),
    "google-double-conversion.txt": (31477043, 564745),
    "google-wuffs.txt": (421511, 10744),
    "ibm-fpgen.txt": (5312826, 102792),
    "lemire-fast-double-parser.txt": (4662544, 94313),
    "lemire-fast-float.txt": (127450, 3299),
    "more-test-cases.txt": (2751, 60),
    "remyoudompheng-fptest-0.txt": (51629834, 1000000),
    "remyoudompheng-fptest-1.txt": (51133002, 1000000),
    "remyoudompheng-fptest-2.txt": (50060842, 1000000),
    "remyoudompheng-fptest-3.txt": (44845192, 885708),
    "tencent-rapidjson.txt": (148425, 3563),
    "ulfjack-ryu.txt": (34351964, 599458),
}
LEMIRE_COMMIT = "0e2b5d163d4074cc0bde2acdaae78546d6e5c5f1"
LEMIRE_URL = f"https://raw.githubusercontent.com/lemire/simple_fastfloat_benchmark/{LEMIRE_COMMIT}/data/"
LEMIRE = {"canada.txt": (2138804, 111126), "mesh.txt": (635065, 73019)}
DECIMAL = re.compile(r"[+-]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?")
SPELLINGS = {"nan", "inf", "infinity"}
EXAMPLES = 10


def bits_of(value: float) -> int:
    return struct.unpack("<Q", struct.pack("<d", value))[0]


def shortest(token: str) -> tuple[int, str, int]:
    """Sign, significant digits and the power of ten of their last digit, with trailing zeros dropped."""
    sign, digits, exponent = Decimal(token).as_tuple()
    text = "".join(map(str, digits)).lstrip("0")
    if not text:
        return sign, "0", 0
    stripped = text.rstrip("0")
    return sign, stripped, exponent + len(text) - len(stripped)


def frame(xs: list[str]) -> str:
    return cc.con_frame(xs)


def alone(readcon, token: str) -> str:
    try:
        readcon.read_con_string(frame([token]))
    except OSError as error:
        return str(error).split(":")[0]
    return "accepted"


def check(readcon, name: str, cases: list[tuple[str, int | None]], work: Path) -> dict:
    """``cases`` holds each string with its expected bits, or ``None`` where CPython's ``float`` decides."""
    start = time.perf_counter()
    finite, infinite, spelled, other = [], [], [], []
    for token, bits in cases:
        if bits is None:
            bits = bits_of(float(token))
        if token.lstrip("+-").lower() in SPELLINGS:
            spelled.append(token)
        elif not DECIMAL.fullmatch(token):
            other.append(token)
        elif (bits >> 52) & 0x7FF == 0x7FF:
            infinite.append((token, bits))
        else:
            finite.append((token, bits))
    xs = [token for token, _ in finite]
    want = np.array([bits for _, bits in finite], dtype=np.uint64)
    text = frame(xs)
    path = work / (Path(name).stem + ".con")
    path.write_text(text)
    positions = np.asarray(readcon.read_positions(str(path), 1))
    from_file = positions[0, :, 0].view(np.uint64)
    frames = readcon.read_con_string(text)
    from_text = frames[0].coords_array()[:, 0].view(np.uint64)
    out = {
        "strings": len(cases),
        "finite_decimals": len(finite),
        "read_arrays_bit_mismatches": int((from_file != want).sum()),
        "read_con_string_bit_mismatches": int((from_text != want).sum()),
    }
    wrong = np.flatnonzero((from_file != want) | (from_text != want))[:EXAMPLES]
    out["mismatch_examples"] = [
        {
            "string": xs[i][:80],
            "expected": f"{int(want[i]):016x}",
            "read_arrays": f"{int(from_file[i]):016x}",
            "read_con_string": f"{int(from_text[i]):016x}",
        }
        for i in wrong
    ]
    path.unlink()

    printed = [
        line.split()[0] for line in readcon.write_con_string(frames).splitlines()[11:]
    ]
    values = from_text.view(np.float64)
    if len(printed) != len(values):
        raise SystemExit(
            f"{name}: the writer printed {len(printed)} rows for {len(values)} atoms"
        )
    seen, round_trip, longer, other_digits, examples = set(), 0, 0, 0, []
    for token, value, raw in zip(printed, values.tolist(), from_text.tolist()):
        if raw in seen:
            continue
        seen.add(raw)
        back = bits_of(float(token)) == raw
        mine, best = shortest(token), shortest(repr(value))
        round_trip += not back
        longer += len(mine[1]) > len(best[1])
        other_digits += len(mine[1]) == len(best[1]) and mine != best
        if (not back or mine != best) and len(examples) < EXAMPLES:
            examples.append({"value": repr(value), "printed": token})
    out["writer"] = {
        "distinct_values": len(seen),
        "round_trip_failures": round_trip,
        "longer_than_shortest": longer,
        "same_length_other_digits": other_digits,
        "examples": examples,
    }

    verdicts = {}
    for token, _ in infinite:
        verdict = alone(readcon, token)
        verdicts.setdefault(("infinite", verdict), []).append(token)
    for token in spelled:
        verdict = alone(readcon, token)
        verdicts.setdefault(("non_finite_spelling", verdict), []).append(token)
    for token in other:
        verdict = alone(readcon, token)
        verdicts.setdefault(("not_a_decimal", verdict), []).append(token)
    out["infinite_expected"] = len(infinite)
    out["non_finite_spellings"] = len(spelled)
    out["not_a_con_decimal"] = len(other)
    out["alone"] = [
        {
            "expected": kind,
            "reader": verdict,
            "count": len(tokens),
            "examples": [t[:80] for t in tokens[:EXAMPLES]],
        }
        for (kind, verdict), tokens in sorted(verdicts.items())
    ]
    out["seconds"] = round(time.perf_counter() - start, 1)
    return out


def main() -> int:
    import readcon

    work = cc.WORK / "decimals"
    work.mkdir(parents=True, exist_ok=True)
    downloads, results = [], {}
    for name, (size, lines) in TAO.items():
        path, got = cc.fetch(TAO_URL + name, f"tao-{name}", size=size)
        rows = path.read_text().splitlines()
        if len(rows) != lines:
            raise SystemExit(f"{name}: {len(rows)} lines, the README counts {lines}")
        got["lines"] = len(rows)
        downloads.append(got)
        results[name] = check(
            readcon, name, [(row[31:], int(row[14:30], 16)) for row in rows], work
        )
        print(
            name,
            {
                key: results[name][key]
                for key in ("strings", "read_arrays_bit_mismatches")
            },
            flush=True,
        )
    for name, (size, lines) in LEMIRE.items():
        path, got = cc.fetch(LEMIRE_URL + name, f"lemire-{name}", size=size)
        rows = path.read_text().splitlines()
        if len(rows) != lines:
            raise SystemExit(f"{name}: {len(rows)} lines, the benchmark reads {lines}")
        got["lines"] = len(rows)
        downloads.append(got)
        results[name] = check(readcon, name, [(row, None) for row in rows], work)
        print(
            name,
            {
                key: results[name][key]
                for key in ("strings", "read_arrays_bit_mismatches")
            },
            flush=True,
        )

    total = {
        key: sum(row[key] for row in results.values())
        for key in (
            "strings",
            "finite_decimals",
            "read_arrays_bit_mismatches",
            "read_con_string_bit_mismatches",
            "infinite_expected",
            "non_finite_spellings",
            "not_a_con_decimal",
        )
    }
    for key in (
        "distinct_values",
        "round_trip_failures",
        "longer_than_shortest",
        "same_length_other_digits",
    ):
        total[f"writer_{key}"] = sum(row["writer"][key] for row in results.values())
    for kind in ("infinite", "non_finite_spelling", "not_a_decimal"):
        verdicts = {}
        for row in results.values():
            for entry in row["alone"]:
                if entry["expected"] == kind:
                    verdicts[entry["reader"]] = (
                        verdicts.get(entry["reader"], 0) + entry["count"]
                    )
        total[f"{kind}_verdicts"] = verdicts
    cc.write_record(
        "decimals",
        {
            "claim": "The reader rounds each decimal to the nearest binary64 value and rejects a decimal past "
            "binary64; the default writer prints the shortest decimal that reads back as the same value.",
            "decimal_grammar": DECIMAL.pattern,
            "total": total,
            "files": results,
        },
        sources=downloads,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
