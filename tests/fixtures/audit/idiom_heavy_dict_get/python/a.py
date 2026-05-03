def coalesce(d, key):
    return d.get(key, "default")


def fallback(d, key):
    return d.get(key, [])
