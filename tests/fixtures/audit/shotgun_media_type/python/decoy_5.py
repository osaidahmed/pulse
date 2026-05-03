def fnv1a(text):
    h = 0xcbf29ce484222325
    for byte in text.encode("utf-8"):
        h = ((h ^ byte) * 0x100000001b3) & 0xffffffffffffffff
    return h
