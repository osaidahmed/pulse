def handle(media_type, args):
    if media_type == "tv":
        run_tv(args)
        return 0
    if media_type == "movie":
        run_movie(args)
        return 0
    return 1


def run_tv(a):
    return a


def run_movie(a):
    return a
