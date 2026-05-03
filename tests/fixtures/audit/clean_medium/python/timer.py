import time


class Timer:
    def __enter__(self):
        self.start = time.monotonic()
        return self

    def __exit__(self, *_):
        self.elapsed = time.monotonic() - self.start
