def fetch(cache, key, source):
    cached = cache.get(key)
    if cached is not None:
        return cached
    value = source.fetch(key)
    cache.set(key, value)
    return value
