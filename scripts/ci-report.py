#!/usr/bin/env python3
"""Expose a bounded command-log tail through GitHub Checks annotations.

Full output stays in the job log. This also supports environments which can
reach api.github.com but cannot download the redirected Actions log archive.
The shell wrapper, not this reporter, preserves the command's exit status.
"""
import pathlib
import sys


def escape(message):
    return message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def report(log, status):
    # Do not allow a log line to inject a second workflow command.
    for line in log.splitlines():
        print("log: " + line if line.startswith("::") else line)
    if status:
        tail = "\n".join(log.splitlines()[-600:])[-24000:]
        chunks = [tail[i : i + 6000] for i in range(0, len(tail), 6000)] or ["No command output"]
        for index, chunk in enumerate(chunks, 1):
            print(f"::error title=Command failed - tail {index} of {len(chunks)}::{escape(chunk)}")
    else:
        tail = "\n".join(log.splitlines()[-25:])[-6000:] or "Command succeeded without output"
        print(f"::notice title=Command succeeded::{escape(tail)}")


if __name__ == "__main__":
    report(pathlib.Path(sys.argv[1]).read_text(errors="replace"), int(sys.argv[2]))
