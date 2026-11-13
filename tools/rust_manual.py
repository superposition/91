# SPDX-License-Identifier: GPL-3.0-only
"""Manual file/PDF-library/process I/O. Parsing and workflow execute in Rust."""
import json
from pathlib import Path
import subprocess
import time
import rust_tooling

DATA = Path(__file__).parent / "data"
COPY = json.loads((DATA / "manual-copy.json").read_text(encoding="utf-8"))


class Adapter:
    def __init__(self, owner):
        self.owner = owner

    def __call__(self, action):
        kind, args = action["kind"], action["args"]
        path = self.owner.get("Path", Path)
        if kind == "read":
            return list(path(args[0]).read_bytes())
        if kind == "read_pdf":
            return path(args[0]).read_bytes().hex()
        if kind == "read_text":
            return path(args[0]).read_text(encoding="utf-8")
        if kind == "write_text":
            kwargs = dict(encoding="utf-8")
            if args[2] == "lf":
                kwargs["newline"] = "\n"
            path(args[0]).write_text(args[1], **kwargs)
        elif kind == "exists":
            return path(args[0]).exists()
        elif kind == "unlink":
            path(args[0]).unlink()
        elif kind == "size":
            return path(args[0]).stat().st_size
        elif kind == "mkdir":
            path(args[0]).mkdir(exist_ok=True)
        elif kind == "as_uri":
            return path(args[0]).as_uri()
        elif kind == "resolved_uri":
            return path(args[0]).resolve().as_uri()
        elif kind == "resolve_parent":
            return str(path(args[0]).resolve().parent)
        elif kind == "set_img_base":
            self.owner["IMG_BASE"] = path(args[0])
        elif kind == "set_embed":
            self.owner["EMBED"] = args[0]
        elif kind == "print":
            print(args[0])
        elif kind == "sleep":
            self.owner.get("time", time).sleep(args[0])
        elif kind == "run":
            process = self.owner.get("subprocess", subprocess)
            kwargs = dict(check=args[1])
            if args[2] is not None:
                kwargs["timeout"] = args[2]
            if args[3]:
                kwargs.update(stdout=process.DEVNULL, stderr=process.DEVNULL)
            process.run(args[0], **kwargs)
        elif kind == "pdf_open":
            try:
                from pypdf import PdfReader
            except ImportError:
                return False
            self.reader = PdfReader(str(args[0]))
            return True
        elif kind == "pdf_count":
            self.pages = self.reader.pages
            return len(self.pages)
        elif kind == "page_text":
            return self.pages[args[0]].extract_text()
        elif kind == "page_metrics":
            page = self.pages[args[0]]
            height = float(page.mediabox.height)
            items = []
            def visit(text, cm, tm, fd, fs):
                items.append([text, cm, tm])
            page.extract_text(visitor_text=visit)
            return [height, items]
        else:
            raise RuntimeError("unknown manual adapter action: " + kind)


def call(method, args, owner=None):
    try:
        return rust_tooling.call("asset.manual." + method, args,
                                 adapter=Adapter(owner) if owner is not None else None)
    except rust_tooling.ToolError as error:
        cls = {"ValueError": ValueError, "IndexError": IndexError,
               "SystemExit": SystemExit, "AssertionError": AssertionError,
               "AttributeError": AttributeError}.get(error.kind)
        if cls is None:
            raise
        raise cls(*([error.args[0]] if error.args[0] else [])) from None


def config(owner):
    edge_env = owner.get("os").environ.get("PERF1_EDGE") if owner.get("os") else None
    return dict(src=str(owner["SRC"]), name=owner.get("NAME", COPY["name"]),
                css=owner.get("CSS", ""), copy=COPY,
                part_colors=owner.get("PART_COLOURS", {}),
                img_base=str(owner["IMG_BASE"]) if owner.get("IMG_BASE") is not None else None,
                embed=owner.get("EMBED", False), edge=owner.get("EDGE", COPY["edge"]),
                edge_env=edge_env, want_pages=owner.get("WANT_PAGES", COPY["want_pages"]),
                here=str(owner.get("HERE", "")), python=owner.get("sys").executable if owner.get("sys") else "")
