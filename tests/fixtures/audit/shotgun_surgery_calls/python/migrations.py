from target import MediaService


class MigrationRunner:
    def __init__(self):
        self.service = MediaService()

    def migrate(self, item):
        self.service.handle(item)

    def rollback(self, item):
        self.service.handle(item)
