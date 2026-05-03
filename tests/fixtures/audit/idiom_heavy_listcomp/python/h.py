def tagged(items, tag):
    return [i for i in items if tag in i.tags]


def words(text):
    return [w for w in text.split() if w.isalpha()]
