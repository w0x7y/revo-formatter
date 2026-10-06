#!/usr/bin/env python3
"""Controlled subprocess for lifecycle, byte rejection and resource tests."""
import os
import signal
import subprocess
import sys
import time

if sys.argv[1:] != ["--indent-width", "2", "--line-width", "80", "-"]:
    sys.stderr.write("unexpected formatter arguments")
    sys.exit(2)
source = sys.stdin.buffer.read()
mode = os.environ.get("REVOFMT_TEST_MODE", "delay")
if mode == "inherited-pipes":
    # The direct child exits successfully while its bounded descendant keeps
    # stdout/stderr open and supplies late output. The test owns PID cleanup.
    child_program = """
import sys, time
try:
    time.sleep(0.8)
    sys.stdout.buffer.write(b'let x = 1\\n')
    sys.stdout.buffer.flush()
    sys.stderr.write('late descendant output')
    sys.stderr.flush()
except BrokenPipeError:
    pass
finally:
    with open(sys.argv[1], 'w') as handle:
        handle.write('done')
"""
    child = subprocess.Popen(
        [sys.executable, "-c", child_program, os.environ["REVOFMT_TEST_CHILD_PID"] + ".done"],
        stdin=subprocess.DEVNULL,
    )
    with open(os.environ["REVOFMT_TEST_CHILD_PID"], "w") as handle:
        handle.write(str(child.pid))
    sys.exit(0)
elif mode == "timeout":
    time.sleep(2)
elif mode == "delay":
    time.sleep(0.2)
elif mode == "stderr":
    sys.stderr.write("controlled syntax failure")
    sys.exit(2)
elif mode == "signal":
    os.kill(os.getpid(), signal.SIGTERM)
elif mode == "oversize":
    sys.stdout.buffer.write(b"x" * 300000)
    sys.exit(0)
elif mode == "unrepresentable":
    sys.stdout.buffer.write(b"let x = 'a\nb'\r\n")
    sys.exit(0)
else:
    raise RuntimeError("unknown fixture mode")
sys.stdout.buffer.write(source.replace(b"let x=1", b"let x = 1") + b"\n")
