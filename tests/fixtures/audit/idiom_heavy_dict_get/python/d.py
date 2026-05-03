def title(item):
    return item.get("title", "untitled")


def tags(item):
    return item.get("tags", [])
