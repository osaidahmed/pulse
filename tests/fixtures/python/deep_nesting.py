"""Function with deeply nested control structures (depth > 4)."""


def deeply_nested(data):
    for group in data:
        if group.active:
            for item in group.items:
                if item.valid:
                    for sub in item.children:
                        if sub.ready:
                            process(sub)
    return True


def moderately_nested(data):
    for item in data:
        if item.active:
            for sub in item.children:
                process(sub)
    return True


def process(x):
    pass
