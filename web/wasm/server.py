#!/usr/bin/env python3
"""Local static demo server with the isolation headers required by WASM threads."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class IsolatedHandler(SimpleHTTPRequestHandler):
    extensions_map = {**SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm"}

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cross-Origin-Resource-Policy", "same-origin")
        super().end_headers()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--bind", default="127.0.0.1")
    args = parser.parse_args()
    handler = partial(IsolatedHandler, directory=str(Path(__file__).resolve().parent))
    with ThreadingHTTPServer((args.bind, args.port), handler) as server:
        print(f"Threaded WASM demo: http://{args.bind}:{args.port}/", flush=True)
        server.serve_forever()
