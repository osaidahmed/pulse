from deps import validate, normalize, persist, notify, log_event, emit_metric


class MediaService:
    def handle(self, item):
        validate(item)
        normalize(item)
        persist(item)
        notify(item)
        log_event(item)
        emit_metric(item)
