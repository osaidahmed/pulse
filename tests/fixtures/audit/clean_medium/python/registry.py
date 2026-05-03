_HANDLERS = {}


def register(name):
    def wrap(fn):
        _HANDLERS[name] = fn
        return fn
    return wrap
