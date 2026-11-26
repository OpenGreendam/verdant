import ctypes as c
import errno
import os
import socket
import sys


class ArgCompare(c.Structure):
    _fields_ = [("arg", c.c_uint), ("op", c.c_int),
                ("value", c.c_uint64), ("value2", c.c_uint64)]


seccomp = c.CDLL("libseccomp.so.2")
seccomp.seccomp_init.argtypes = [c.c_uint32]
seccomp.seccomp_init.restype = c.c_void_p
seccomp.seccomp_syscall_resolve_name.argtypes = [c.c_char_p]
seccomp.seccomp_rule_add_array.argtypes = [c.c_void_p, c.c_uint32, c.c_int,
                                         c.c_uint, c.POINTER(ArgCompare)]
seccomp.seccomp_load.argtypes = [c.c_void_p]
ctx = seccomp.seccomp_init(0x7FFF0000)
if not ctx:
    raise SystemExit("Could not initialize the verification sandbox")
socket_call = seccomp.seccomp_syscall_resolve_name(b"socket")
if socket_call < 0:
    raise SystemExit("Could not resolve the socket syscall")
for family in (socket.AF_INET, socket.AF_INET6, socket.AF_PACKET):
    same_family = ArgCompare(0, 4, family, 0)
    if seccomp.seccomp_rule_add_array(ctx, 0x00050000 | errno.EPERM, socket_call, 1, c.byref(same_family)) != 0:
        raise SystemExit("Could not configure the verification sandbox")
if seccomp.seccomp_load(ctx) != 0:
    raise SystemExit("Could not load the verification sandbox")
try:
    socket.socket(socket.AF_INET, socket.SOCK_STREAM).close()
except PermissionError:
    pass
else:
    raise SystemExit("The verifier can still create IPv4 sockets")
os.execve(sys.argv[1], sys.argv[1:], {"HOME": os.environ["HOME"], "PATH": ""})
