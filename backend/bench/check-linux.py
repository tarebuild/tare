"""Probe the exact io_uring setup syscall required by the pinned Agave validator."""
import ctypes
import errno
import os
import platform
import sys

if platform.system() != "Linux" or platform.machine() not in ("x86_64", "amd64"):
    raise SystemExit("The pinned validator image requires Linux x86_64.")
libc = ctypes.CDLL(None, use_errno=True)
# Linux x86_64 __NR_io_uring_setup; io_uring_params is 120 bytes.
parameters = ctypes.create_string_buffer(120)
fd = libc.syscall(ctypes.c_long(425), ctypes.c_uint(1), ctypes.byref(parameters))
if fd < 0:
    code = ctypes.get_errno()
    print(f"Validator preflight failed: io_uring_setup: {os.strerror(code)} (errno {code}).", file=sys.stderr)
    if code in (errno.EPERM, errno.EACCES):
        print("The container/host policy denies io_uring. Recreate this trusted test container with --security-opt seccomp=unconfined, no published ports, and no wallet/secrets mounts; see bench/lead-run.md. A host-level io_uring restriction may still need an administrator change.", file=sys.stderr)
    elif code == errno.ENOSYS:
        print("This kernel does not implement io_uring; use a supported Linux kernel.", file=sys.stderr)
    raise SystemExit(1)
os.close(fd)
print("Validator preflight: io_uring_setup is available.")
