def fetch(cache, key, api):
    cached = cache.get(key)
    if cached is not None:
        return cached
    value = api.fetch(key)
    cache.set(key, value)
    return value
