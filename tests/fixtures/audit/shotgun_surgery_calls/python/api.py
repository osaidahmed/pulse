from target import MediaService


class ApiHandler:
    def __init__(self):
        self.service = MediaService()

    def post(self, item):
        self.service.handle(item)

    def put(self, item):
        self.service.handle(item)

    def patch(self, item):
        self.service.handle(item)
