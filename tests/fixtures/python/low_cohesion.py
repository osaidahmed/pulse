"""Class with low cohesion — methods don't share state (future: LCOM4)."""


class KitchenSink:
    def __init__(self):
        self.users = []
        self.orders = []
        self.logs = []
        self.config = {}

    def add_user(self, user):
        self.users.append(user)

    def remove_user(self, user_id):
        self.users = [u for u in self.users if u.id != user_id]

    def get_user(self, user_id):
        return next((u for u in self.users if u.id == user_id), None)

    def create_order(self, order):
        self.orders.append(order)

    def cancel_order(self, order_id):
        self.orders = [o for o in self.orders if o.id != order_id]

    def get_order(self, order_id):
        return next((o for o in self.orders if o.id == order_id), None)

    def log_event(self, event):
        self.logs.append(event)

    def get_logs(self):
        return self.logs

    def clear_logs(self):
        self.logs = []

    def set_config(self, key, value):
        self.config[key] = value

    def get_config(self, key):
        return self.config.get(key)
