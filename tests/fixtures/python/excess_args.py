"""Functions with too many parameters."""


def create_user(first_name, last_name, email, phone, address, city, state, zipcode):
    return {
        "first_name": first_name,
        "last_name": last_name,
        "email": email,
        "phone": phone,
        "address": address,
        "city": city,
        "state": state,
        "zipcode": zipcode,
    }


def simple_func(a, b):
    return a + b


class UserService:
    def __init__(self, db, cache, logger, mailer, validator, scheduler, p7, p8, p9):
        self.db = db
        self.cache = cache
        self.logger = logger
        self.mailer = mailer
        self.validator = validator
        self.scheduler = scheduler

    def get_user(self, user_id):
        return self.db.get(user_id)
