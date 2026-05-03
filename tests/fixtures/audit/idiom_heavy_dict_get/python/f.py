def status(record):
    return record.get("status", "pending")


def retries(record):
    return record.get("retries", 0)
