from contextlib import contextmanager


@contextmanager
def silenced(stream):
    saved = stream.write
    stream.write = lambda *_a, **_k: 0
    try:
        yield stream
    finally:
        stream.write = saved
