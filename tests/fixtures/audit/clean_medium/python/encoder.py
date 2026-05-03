def encode(payload):
    if isinstance(payload, bytes):
        return payload
    return str(payload).encode("utf-8")
