def env(settings):
    return settings.get("env", "dev")


def debug(settings):
    return settings.get("debug", False)
