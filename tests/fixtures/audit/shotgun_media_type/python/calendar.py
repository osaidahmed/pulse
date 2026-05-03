def emit_events(media_type, items):
    if media_type == "tv":
        return [build_episode(i) for i in items]
    if media_type == "season":
        return [build_season(i) for i in items]
    return []


def build_episode(i):
    return i


def build_season(i):
    return i
