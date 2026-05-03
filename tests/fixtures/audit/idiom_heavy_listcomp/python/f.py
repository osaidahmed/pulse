def numeric(items):
    return [int(i) for i in items if str(i).isdigit()]


def lower(items):
    return [s.lower() for s in items if isinstance(s, str)]
