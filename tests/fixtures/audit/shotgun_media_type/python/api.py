def serialize(media_type, obj):
    if media_type == "tv":
        return {"kind": "tv", "id": obj.id}
    if media_type == "movie":
        return {"kind": "movie", "id": obj.id}
    return {"kind": "unknown"}
