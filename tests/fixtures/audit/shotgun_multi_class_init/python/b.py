class BImporter:
    def __init__(self, user, mode):
        self.user = user
        self.mode = mode
        self.warnings = []
        self.existing = {}

    def import_data(self):
        return self.warnings
