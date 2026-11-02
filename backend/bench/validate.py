"""Validate the published schemas and the retained replay payload."""
import base64
import copy
import gzip
import hashlib
import io
import json
import pathlib
import sys
import jsonschema

root = pathlib.Path(__file__).resolve().parent.parent
name, document = sys.argv[1:]
schema = json.loads((root / "contract/schema" / f"{name}.json").read_text(encoding="utf-8"))
value = json.loads(pathlib.Path(document).read_text(encoding="utf-8"))
projection = copy.deepcopy(value)
replay_paths = []

def project(node, path="$"):
    if isinstance(node, dict):
        if "replay" in node and {"builds", "proof", "instructions", "schemaVersion"} <= node.keys():
            replay = node["replay"]
            jsonschema.validate(replay, {
                "type": "object", "additionalProperties": False,
                "required": ["encoding", "sha256", "data"],
                "properties": {
                    "encoding": {"const": "gzip+base64"},
                    "sha256": {"type": "string", "pattern": "^[a-f0-9]{64}$"},
                    "data": {"type": "string"},
                },
            })
            compressed = base64.b64decode(replay["data"], validate=True)
            with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
                payload = stream.read(256 * 1024 * 1024 + 1)
            if len(payload) > 256 * 1024 * 1024:
                raise ValueError(f"{path}.replay exceeds 256 MiB")
            if hashlib.sha256(payload).hexdigest() != replay["sha256"]:
                raise ValueError(f"{path}.replay SHA-256 differs")
            if node["proof"]["corpus"]["hash"] != replay["sha256"]:
                raise ValueError(f"{path}.proof corpus hash differs from replay")
            json.loads(payload)
            replay_paths.append(path + ".replay")
        for key, child in node.items():
            project(child, f"{path}.{key}")
    elif isinstance(node, list):
        for index, child in enumerate(node):
            project(child, f"{path}[{index}]")

project(projection)
errors = list(jsonschema.Draft202012Validator(schema).iter_errors(projection))
if errors:
    error = jsonschema.exceptions.best_match(errors)
    raise SystemExit(f"{document}: {'.'.join(map(str, error.absolute_path))}: {error.message}")
print(f"{document}: published {name} schema passed" +
      (f"; {len(replay_paths)} replay payload(s) validated and retained" if replay_paths else ""))
