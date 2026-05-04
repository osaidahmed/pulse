from target import MediaService


class WebhookListener:
    def __init__(self):
        self.service = MediaService()

    def receive(self, item):
        self.service.handle(item)

    def replay(self, item):
        self.service.handle(item)
