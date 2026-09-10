#!/usr/bin/env python3
"""Reject generated build/acceptance output in the Git index."""
import subprocess

paths = subprocess.check_output(["git", "ls-files", "-z"], text=True).split("\0")
outputs = ("assets/ui/", "evidence/", "artifacts/", "target/", "dist/", "node_modules/")
tracked = [path for path in paths if path.startswith(outputs) or "__pycache__" in path.split("/")]
if tracked:
    raise SystemExit("Generated output must not be committed:\n" + "\n".join(tracked))
print("No generated UI bundles or acceptance output in the Git index.")
