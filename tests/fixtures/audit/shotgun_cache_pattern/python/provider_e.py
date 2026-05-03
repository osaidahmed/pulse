def fetch(cache, key, store):
    cached = cache.get(key)
    if cached is not None:
        return cached
    value = store.fetch(key)
    cache.set(key, value)
    return value
