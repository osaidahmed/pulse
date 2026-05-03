from pathlib import Path


def resolve(p):
    return Path(p).expanduser().resolve()
