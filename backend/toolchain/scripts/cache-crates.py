"""Fill missing public crates.io archives for the Windows offline Cargo run."""
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import urllib.request

home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
cache = home / "registry/cache/index.crates.io-1949cf8c6b5b557f"
cache.mkdir(parents=True, exist_ok=True)
requests = set()
if len(sys.argv) == 3 and sys.argv[1] == "--index":
    name = sys.argv[2]
    if not re.fullmatch(r"[A-Za-z0-9_-]+", name):
        raise SystemExit("Invalid crate name")
    prefix = name[:2] + "/" + name[2:4] if len(name) >= 4 else ("3/" + name[0] if len(name) == 3 else str(len(name)))
    response = urllib.request.urlopen(f"https://index.crates.io/{prefix}/{name}")
    entries = response.read().splitlines()
    data = b"\x03\x02\x00\x00\x00" + ("etag: " + response.headers["ETag"]).encode() + b"\0"
    for line in entries:
        entry = json.loads(line)
        data += entry["vers"].encode() + b"\0" + line + b"\0"
    target = home / "registry/index/index.crates.io-1949cf8c6b5b557f/.cache" / prefix / name
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_suffix(".download")
    temporary.write_bytes(data)
    temporary.replace(target)
    print(f"Cached sparse index {name}")
    raise SystemExit(0)
if sys.argv[1:] == ["--transcripts"]:
    for path in (Path(__file__).resolve().parents[2] / "bench/transcripts").rglob("*"):
        if path.is_file() and path.suffix in (".txt", ".log"):
            text = path.read_text(encoding="utf-8", errors="replace")
            requests.update(re.findall(r"failed to download `([A-Za-z0-9_-]+) v([^`]+)`", text))
else:
    requests.update(tuple(argument.rsplit("@", 1)) for argument in sys.argv[1:])
for name, version in sorted(requests):
    if not re.fullmatch(r"[A-Za-z0-9_-]+", name) or not re.fullmatch(r"[0-9A-Za-z.+_-]+", version):
        raise SystemExit("Invalid crate name/version")
    target = cache / f"{name}-{version}.crate"
    if target.exists():
        continue
    prefix = name[:2] + "/" + name[2:4] if len(name) >= 4 else ("3/" + name[0] if len(name) == 3 else str(len(name)))
    index = home / "registry/index/index.crates.io-1949cf8c6b5b557f/.cache" / prefix / name
    record = None
    if index.exists():
        for part in index.read_bytes().split(b"\0"):
            if part.startswith(b"{"):
                entry = json.loads(part)
                if entry["vers"] == version:
                    record = entry
                    break
    checksum = record["cksum"] if record else json.load(urllib.request.urlopen(f"https://crates.io/api/v1/crates/{name}/{version}"))["version"]["checksum"]
    data = urllib.request.urlopen(f"https://static.crates.io/crates/{name}/{name}-{version}.crate").read()
    if hashlib.sha256(data).hexdigest() != checksum:
        raise SystemExit(f"Checksum mismatch: {name}@{version}")
    temporary = target.with_suffix(".download")
    temporary.write_bytes(data)
    temporary.replace(target)
    print(f"Cached {name}@{version}", flush=True)
