from target import MediaService


class WorkerJob:
    def __init__(self):
        self.service = MediaService()

    def execute(self, item):
        self.service.handle(item)

    def retry(self, item):
        self.service.handle(item)
