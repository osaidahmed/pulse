def host(config):
    return config.get("host", "localhost")


def port(config):
    return config.get("port", 8080)
