def name(meta):
    return meta.get("name", "anonymous")


def perms(meta):
    return meta.get("permissions", [])
