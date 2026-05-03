PI_DIGITS = "3.141592653589793"


def truncate(n):
    return PI_DIGITS[: 2 + n] if n > 0 else "3"
