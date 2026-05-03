def headers(request):
    return request.get("headers", {})


def body(request):
    return request.get("body", b"")
