"""CPU model and platform label for bench provenance."""

from __future__ import annotations

import platform
from pathlib import Path


def normalize_cpu(raw: str) -> str:
    """Drop trademark marks. The model name itself stays."""
    cleaned = " ".join(raw.replace("(R)", "").replace("(TM)", "").replace("®", "").split())
    return cleaned or "unknown"


def cpu_record(model: str, family: str, model_id: str, stepping: str) -> str:
    """Model name plus family, model, and stepping when the kernel printed them."""
    name = normalize_cpu(model) if model else "unknown"
    parts = []
    if family:
        parts.append(f"family {family}")
    if model_id:
        parts.append(f"model {model_id}")
    if stepping:
        parts.append(f"stepping {stepping}")
    if not parts:
        return name
    if name == "unknown":
        return " ".join(parts)
    return f"{name} {' '.join(parts)}"


def _field(text: str, key: str) -> str:
    for line in text.splitlines():
        if ":" not in line:
            continue
        left, value = line.split(":", 1)
        if left.strip().lower() == key:
            return value.strip()
    return ""


def cpu_model() -> str:
    """Model name, family, model, and stepping from ``/proc/cpuinfo``."""
    try:
        text = Path("/proc/cpuinfo").read_text(encoding="utf-8")
    except OSError:
        return "unknown"
    model = _field(text, "model name")
    family = _field(text, "cpu family")
    model_id = _field(text, "model")
    stepping = _field(text, "stepping")
    if not any((model, family, model_id, stepping)):
        return "unknown"
    return cpu_record(model, family, model_id, stepping)


def strip_host_token(label: str, node: str) -> str:
    """Drop dash-separated components that equal the machine name."""
    node = node.strip()
    if not node:
        return label
    return "-".join(part for part in label.split("-") if part != node)


def platform_label() -> str:
    """System, machine, and libc, without the kernel release.

    ``platform.platform()`` embeds ``uname -r``. A localversion on that
    release is machine-specific, so a rerun would record it again. A
    component equal to the machine name is still removed.
    """
    system = platform.system().strip() or "unknown"
    machine = platform.machine().strip() or "unknown"
    libc = ""
    full = platform.platform()
    marker = "-with-"
    if marker in full:
        libc = full.rsplit(marker, 1)[1]
    parts = [system, machine]
    if libc:
        parts.extend(["with", libc])
    return strip_host_token("-".join(parts), platform.node())
