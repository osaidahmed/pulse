ROUTES = {"home": "/", "about": "/about"}


def resolve(name):
    return ROUTES.get(name, "/")
