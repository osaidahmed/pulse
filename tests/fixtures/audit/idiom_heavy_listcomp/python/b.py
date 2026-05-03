def names(rows):
    return [r.name for r in rows if r.active]


def keys(d):
    return [k for k in d.keys() if k.startswith("_")]
