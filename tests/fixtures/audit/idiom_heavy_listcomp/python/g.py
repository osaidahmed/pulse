def addressed(rows):
    return [r for r in rows if r.address is not None]


def nonempty(values):
    return [v for v in values if v]
