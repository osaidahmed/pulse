"""Module-level conditional logic and deep global nesting."""

import os
import sys

DEBUG = os.environ.get("DEBUG", "false") == "true"

if sys.platform == "win32":
    PATH_SEP = "\\"
    if os.environ.get("CYGWIN"):
        PATH_SEP = "/"
        if os.environ.get("MSYSTEM"):
            PATH_SEP = "/"
else:
    PATH_SEP = "/"

if DEBUG:
    import logging
    logging.basicConfig(level=logging.DEBUG)


def clean_function():
    return 42
