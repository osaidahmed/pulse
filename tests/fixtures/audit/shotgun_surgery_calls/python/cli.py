from target import MediaService


class CliRunner:
    def __init__(self):
        self.service = MediaService()

    def run(self, item):
        self.service.handle(item)

    def dry_run(self, item):
        self.service.handle(item)
