from target import MediaService


class AdminPanel:
    def __init__(self):
        self.service = MediaService()

    def approve(self, item):
        self.service.handle(item)

    def reject(self, item):
        self.service.handle(item)
