"""Filesystem helpers shared by the scanner and the UI."""

import ctypes
import os
import sys
from ctypes import wintypes

from .config import DEFAULT_CLUSTER_SIZE

SIZE_UNITS = ("B", "KB", "MB", "GB", "TB", "PB", "EB")


def get_cluster_size(path):
    try:
        if sys.platform == "win32":
            sectors_per_cluster = wintypes.DWORD()
            bytes_per_sector = wintypes.DWORD()
            free = wintypes.DWORD()
            total = wintypes.DWORD()
            abs_path = os.path.abspath(path)
            drive = _volume_root(abs_path)
            if ctypes.windll.kernel32.GetDiskFreeSpaceW(
                ctypes.c_wchar_p(drive),
                ctypes.pointer(sectors_per_cluster),
                ctypes.pointer(bytes_per_sector),
                ctypes.pointer(free),
                ctypes.pointer(total)
            ):
                cluster_size = sectors_per_cluster.value * bytes_per_sector.value
                if cluster_size:
                    return cluster_size
        else:
            stat = os.statvfs(path)
            if stat.f_frsize:
                return stat.f_frsize
    except Exception:
        pass
    return DEFAULT_CLUSTER_SIZE


def _volume_root(abs_path):
    """Build a root path acceptable to GetDiskFreeSpaceW, including UNC shares."""
    drive = os.path.splitdrive(abs_path)[0]
    if drive.startswith("\\\\"):
        return drive + "\\"
    return (drive or "C:") + "\\"


def format_size(size):
    value = float(size)
    for unit in SIZE_UNITS:
        if abs(value) < 1024 or unit == SIZE_UNITS[-1]:
            digits = 0 if unit == "B" else 1
            return f"{value:.{digits}f} {unit}"
        value /= 1024
