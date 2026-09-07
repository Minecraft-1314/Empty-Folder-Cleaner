"""Filesystem helpers shared by the scanner and the UI."""

import ctypes
import os
import sys

from .config import DEFAULT_CLUSTER_SIZE


def get_cluster_size(path):
    try:
        if sys.platform == "win32":
            sectors_per_cluster = ctypes.c_ulonglong()
            bytes_per_sector = ctypes.c_ulonglong()
            free = ctypes.c_ulonglong()
            total = ctypes.c_ulonglong()
            abs_path = os.path.abspath(path)
            drive = os.path.splitdrive(abs_path)[0] + "\\"
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


def format_size(size):
    units = (("B", 0), ("KB", 1), ("MB", 1), ("GB", 2))
    value = float(size)
    for unit, digits in units:
        if value < 1024 or unit == units[-1][0]:
            return f"{value:.{digits}f} {unit}"
        value /= 1024
