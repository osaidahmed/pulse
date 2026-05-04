from target import MediaService


class Scheduler:
    def __init__(self):
        self.service = MediaService()

    def enqueue(self, item):
        self.service.handle(item)

    def dispatch(self, item):
        self.service.handle(item)
