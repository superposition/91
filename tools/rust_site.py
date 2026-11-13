# SPDX-License-Identifier: GPL-3.0-only
"""Thin bindings for maintained Rust metadata and static-site transforms."""
if __package__:
    from . import rust_tooling
else:
    import rust_tooling


def transform(method, *args, adapter=None):
    try:
        return rust_tooling.call("asset." + method, args, adapter=adapter)
    except rust_tooling.ToolError as error:
        classes = {"ValueError": ValueError, "AssertionError": AssertionError,
                   "KeyError": KeyError, "NameError": NameError,
                   "AttributeError": AttributeError, "IndexError": IndexError,
                   "SyntaxError": SyntaxError, "SystemExit": SystemExit}
        raise classes.get(error.kind, ValueError)(str(error)) from None
