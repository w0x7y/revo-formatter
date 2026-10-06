#!/usr/bin/env python3
"""Controlled subprocess for lifecycle, byte rejection and resource tests."""
import os
import sys
import time

if sys.argv[1:] != ["--indent-width", "2", "--line-width", "80", "-"]:
    sys.stderr.write("unexpected formatter arguments")
    sys.exit(2)
source = sys.stdin.buffer.read()
mode = os.environ.get("REVOFMT_TEST_MODE", "delay")
if mode == "timeout":
    time.sleep(2)
elif mode == "delay":
    time.sleep(0.2)
elif mode == "stderr":
    sys.stderr.write("controlled syntax failure")
    sys.exit(2)
elif mode == "oversize":
    sys.stdout.buffer.write(b"x" * 300000)
    sys.exit(0)
elif mode == "unrepresentable":
    sys.stdout.buffer.write(b"let x = 'a\nb'\r\n")
    sys.exit(0)
else:
    raise RuntimeError("unknown fixture mode")
sys.stdout.buffer.write(source.replace(b"let x=1", b"let x = 1") + b"\n")
