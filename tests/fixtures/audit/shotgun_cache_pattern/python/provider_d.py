def fetch(cache, key, http):
    cached = cache.get(key)
    if cached is not None:
        return cached
    value = http.fetch(key)
    cache.set(key, value)
    return value
