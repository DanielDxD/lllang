#!/usr/bin/env python3
"""Minimal file-backed LLLang registry host (Phase 1.5)."""
from __future__ import annotations

import json
import os
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

ROOT = Path(__file__).resolve().parent.parent / "data"
ROOT.mkdir(parents=True, exist_ok=True)
PORT = int(os.environ.get("LANG_REGISTRY_PORT", "4873"))


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt: str, *args) -> None:
        print(f"[registry] {self.address_string()} {fmt % args}")

    def _send(self, code: int, body: bytes, ctype: str = "application/octet-stream") -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:
        u = urlparse(self.path)
        if u.path == "/health":
            return self._send(200, b"ok\n", "text/plain")
        if u.path == "/v1/search":
            q = (parse_qs(u.query).get("q") or [""])[0].lower()
            pkgs = []
            if ROOT.is_dir():
                for p in sorted(ROOT.iterdir()):
                    if p.is_dir() and (not q or q in p.name.lower()):
                        pkgs.append({"id": p.name, "versions": [c.name for c in p.iterdir() if c.is_dir()]})
            return self._send(200, json.dumps(pkgs).encode(), "application/json")
        if u.path.startswith("/v1/packages/"):
            rel = u.path[len("/v1/packages/") :]
            path = ROOT.joinpath(*rel.split("/"))
            if path.is_file():
                return self._send(200, path.read_bytes())
            return self._send(404, b"not found\n", "text/plain")
        self._send(404, b"not found\n", "text/plain")

    def do_PUT(self) -> None:
        u = urlparse(self.path)
        if not u.path.startswith("/v1/packages/"):
            return self._send(404, b"not found\n", "text/plain")
        rel = u.path[len("/v1/packages/") :]
        path = ROOT.joinpath(*rel.split("/"))
        path.parent.mkdir(parents=True, exist_ok=True)
        length = int(self.headers.get("Content-Length", "0"))
        data = self.rfile.read(length)
        path.write_bytes(data)
        self._send(201, b"created\n", "text/plain")


def main() -> None:
    print(f"LLLang registry host on http://127.0.0.1:{PORT} (data={ROOT})")
    HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()


if __name__ == "__main__":
    main()
