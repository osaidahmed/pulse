import re


PATTERN = re.compile(r"^(?P<key>\w+)=(?P<val>.*)$", re.MULTILINE)


def parse_env(text):
    return {m["key"]: m["val"] for m in PATTERN.finditer(text)}
