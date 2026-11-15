"""Local sampling fixture; transactions still execute on both real SBF builds."""
import argparse
import json
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int, default=7814)
parser.add_argument("--requests", type=Path, required=True)
args = parser.parse_args()
source = Path(__file__).with_name("new-idl-recorded-input.json")
frame = json.loads(source.read_text())["sequences"][0]["transactions"][0]
accounts = {item["address"]: item["account"] for item in frame["preAccounts"]}
requests = []

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method, params = request["method"], request["params"]
        requests.append({"method": method, "params": params})
        args.requests.write_text(json.dumps(requests, indent=2) + "\n", encoding="utf-8")
        if method == "getSignaturesForAddress":
            result = [{"signature": "local-sampling-regression", "slot": 42, "err": None}]
        elif method == "getTransaction":
            result = {"slot": 42, "transaction": [frame["transactionBase64"], "base64"],
                      "meta": {"err": {"InstructionError": [0, {"Custom": 101}]},
                               "innerInstructions": [], "loadedAddresses": {"writable": [], "readonly": []}}}
        elif method == "getMultipleAccounts":
            values = []
            for key in params[0]:
                value = accounts.get(key)
                values.append(None if value is None else {
                    "owner": value["owner"], "lamports": value["lamports"],
                    "executable": value["executable"], "rentEpoch": value["rentEpoch"],
                    "data": [value["dataBase64"], "base64"]})
            result = {"context": {"slot": 42}, "value": values}
        else:
            self.send_error(400, "Unexpected fixture method")
            return
        body = json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

print(f"Local mainnet-sampling fixture: http://127.0.0.1:{args.port}", flush=True)
HTTPServer(("127.0.0.1", args.port), Handler).serve_forever()
