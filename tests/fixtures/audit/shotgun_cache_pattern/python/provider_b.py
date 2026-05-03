def fetch(cache, key, remote):
    cached = cache.get(key)
    if cached is not None:
        return cached
    value = remote.fetch(key)
    cache.set(key, value)
    return value
