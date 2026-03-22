"""Realistic Python API service with intentional code smells for testing."""

class UserProfile:
    """Class with too many fields — triggers Large Struct."""
    def __init__(self):
        self.user_id = None
        self.username = None
        self.email = None
        self.first_name = None
        self.last_name = None
        self.phone = None
        self.address = None
        self.city = None
        self.state = None
        self.zip_code = None
        self.country = None
        self.avatar_url = None
        self.bio = None
        self.created_at = None
        self.updated_at = None


class RequestHandler:
    """Handler using short variable names in long functions."""
    def process_request(self, request, config):
        t = config.get("timeout", 30)
        r = config.get("retries", 3)
        h = config.get("headers", {})
        m = request.get("method", "GET")
        u = request.get("url", "")
        b = request.get("body", None)
        p = config.get("proxy", None)
        s = config.get("ssl", True)
        d = request.get("data", {})
        c = config.get("cache", False)
        v = config.get("verbose", False)
        if not u:
            return {"error": "no url"}
        if m == "POST" and not b:
            return {"error": "no body"}
        if t > 60:
            t = 60
        result = self._send(u, m, h, b, t, r, p, s)
        if v:
            self._log(result, d, c)
        return result

    def _send(self, url, method, headers, body, timeout, retries, proxy, ssl):
        return {"status": 200}

    def _log(self, result, data, cache):
        pass


def route_request(action):
    """String-based dispatch — triggers Stringly-Typed Switch."""
    if action == "create_user":
        return handle_create()
    elif action == "delete_user":
        return handle_delete()
    elif action == "update_profile":
        return handle_update()
    elif action == "reset_password":
        return handle_reset()
    elif action == "send_notification":
        return handle_notify()
    elif action == "export_data":
        return handle_export()
    elif action == "import_data":
        return handle_import()
    else:
        return handle_unknown()


def handle_create():
    return "created"

def handle_delete():
    return "deleted"

def handle_update():
    return "updated"

def handle_reset():
    return "reset"

def handle_notify():
    return "notified"

def handle_export():
    return "exported"

def handle_import():
    return "imported"

def handle_unknown():
    return "unknown"
