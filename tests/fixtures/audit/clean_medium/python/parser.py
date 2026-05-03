def parse(line):
    parts = line.split(",")
    return {parts[0]: parts[1]} if len(parts) >= 2 else {}
