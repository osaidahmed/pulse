def memoize_with_capacity(capacity):
    def decorate(fn):
        cache = {}
        order = []

        def wrapped(*args):
            if args in cache:
                return cache[args]
            value = fn(*args)
            cache[args] = value
            order.append(args)
            if len(order) > capacity:
                del cache[order.pop(0)]
            return value
        return wrapped
    return decorate
