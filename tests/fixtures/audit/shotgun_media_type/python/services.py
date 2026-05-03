def dispatch(media_type, payload):
    if media_type == "tv":
        return process_tv(payload)
    if media_type == "season":
        return process_season(payload)
    if media_type == "anime":
        return process_anime(payload)
    return None


def process_tv(p):
    return p


def process_season(p):
    return p


def process_anime(p):
    return p
