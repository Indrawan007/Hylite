#!/usr/bin/env python3
"""Expose a bounded command-log tail through GitHub Checks annotations.

Full output stays in the job log. This also supports environments which can
reach api.github.com but cannot download the redirected Actions log archive.
The shell wrapper, not this reporter, preserves the command's exit status.
"""
import pathlib
import re
import sys
import uuid


def escape(message):
    return message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def report(log, status):
    # The runner can recognize workflow commands inside a line, not only at
    # its start. Suspend parsing while replaying arbitrary compiler/test output.
    token = uuid.uuid4().hex
    print(f"::stop-commands::{token}")
    print(log, end="" if log.endswith("\n") else "\n")
    print(f"::{token}::")
    if status:
        clean = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", log)
        diagnostic = re.search(r"^(?:error(?:\[|:)|failures:)", clean, re.MULTILINE)
        tail = clean[diagnostic.start():] if diagnostic else "\n".join(clean.splitlines()[-600:])
        if len(tail) > 24000:
            tail = tail[:11950] + "\n... middle of diagnostics omitted; see full job log ...\n" + tail[-11950:]
        # Checks API truncates annotation messages at 4096 characters. Keep each
        # decoded chunk below that bound; total diagnostic tail remains below 24,000 characters.
        chunks = [tail[i : i + 3000] for i in range(0, len(tail), 3000)] or ["No command output"]
        for index, chunk in enumerate(chunks, 1):
            print(f"::error title=Command failed - tail {index} of {len(chunks)}::{escape(chunk)}")
    else:
        clean = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", log)
        tail = "\n".join(clean.splitlines()[-25:])[-6000:] or "Command succeeded without output"
        print(f"::notice title=Command succeeded::{escape(tail)}")


if __name__ == "__main__":
    report(pathlib.Path(sys.argv[1]).read_text(errors="replace"), int(sys.argv[2]))
