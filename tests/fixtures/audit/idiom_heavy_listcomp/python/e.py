def ids(rows):
    return [r.id for r in rows if r.is_visible]


def trimmed(strings):
    return [s.strip() for s in strings if s.strip()]
