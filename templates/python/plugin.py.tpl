"""Example LocalReel storage plugin (Python).

It exposes one local directory through the sidecar protocol: the host passes a
token and a JSON config, the plugin prints one ``ready`` line, then answers
stat/list/walk/read over loopback HTTP.
"""

import json
import os
import sys
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TOKEN = os.environ.get("LOCALREEL_PLUGIN_TOKEN", "")
CONFIG = json.loads(os.environ.get("LOCALREEL_PLUGIN_CONFIG", "{}"))
ROOT = os.path.realpath(CONFIG.get("root", ""))
if not CONFIG.get("root"):
    sys.stderr.write("config.root is required\n")
    sys.exit(1)


def file_uri(path):
    normalized = path.replace(os.sep, "/")
    return f"file:///{normalized}" if os.name == "nt" else f"file://{normalized}"


def resolve(uri):
    if uri.startswith("file://"):
        target = uri[len("file://") :]
        if os.name == "nt":
            target = target.lstrip("/").replace("/", "\\")
    else:
        target = os.path.join(ROOT, uri)
    try:
        real = os.path.realpath(target)
        if os.path.commonpath([real, ROOT]) != ROOT:
            return None
        return real
    except ValueError:
        return None


def stat_of(path):
    stat = os.stat(path)
    return {
        "size": stat.st_size,
        "modified": int(stat.st_mtime),
        "is_dir": os.path.isdir(path),
    }


def describe(path):
    stat = os.stat(path)
    return {
        "uri": file_uri(path),
        "name": os.path.basename(path),
        "is_dir": os.path.isdir(path),
        "size": stat.st_size,
        "modified": int(stat.st_mtime),
    }


def walk(directory, recursive, out):
    for name in os.listdir(directory):
        absolute = os.path.join(directory, name)
        if os.path.isdir(absolute):
            if recursive:
                walk(absolute, True, out)
            continue
        out.append(describe(absolute))


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *_args):
        pass

    def send_body(self, status, body, content_type="application/json"):
        if isinstance(body, str):
            body = body.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if TOKEN and self.headers.get("X-LocalReel-Token") != TOKEN:
            return self.send_body(403, "forbidden", "text/plain")
        parsed = urllib.parse.urlparse(self.path)
        query = urllib.parse.parse_qs(parsed.query)
        uri = query.get("uri", [""])[0]
        if parsed.path == "/v1/health":
            return self.send_body(200, '{"ok":true}')
        if parsed.path == "/v1/stat":
            target = resolve(uri)
            if target is None:
                return self.send_body(404, "missing", "text/plain")
            try:
                return self.send_body(200, json.dumps(stat_of(target)))
            except OSError:
                return self.send_body(404, "missing", "text/plain")
        if parsed.path in ("/v1/list", "/v1/walk"):
            target = resolve(uri)
            if target is None:
                return self.send_body(404, "missing", "text/plain")
            entries = []
            try:
                walk(target, parsed.path == "/v1/walk", entries)
            except OSError:
                return self.send_body(404, "missing", "text/plain")
            return self.send_body(200, json.dumps({"entries": entries}))
        if parsed.path == "/v1/read":
            target = resolve(uri)
            if target is None:
                return self.send_body(404, "missing", "text/plain")
            start = int(query.get("start", ["0"])[0])
            length = int(query.get("len", ["0"])[0])
            try:
                with open(target, "rb") as handle:
                    handle.seek(start)
                    return self.send_body(
                        200,
                        handle.read(max(0, length)),
                        "application/octet-stream",
                    )
            except OSError:
                return self.send_body(404, "missing", "text/plain")
        return self.send_body(404, "unknown route", "text/plain")


def main():
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    print(json.dumps({"ready": True, "port": server.server_address[1]}), flush=True)

    def watch_stdin():
        sys.stdin.buffer.read()
        os._exit(0)

    threading.Thread(target=watch_stdin, daemon=True).start()
    server.serve_forever()


if __name__ == "__main__":
    main()
