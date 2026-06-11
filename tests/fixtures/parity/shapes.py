def flat_calls():
    a = 1
    b = 2
    c = a + b
    return c


def pick_branch(x):
    if x > 10:
        return 3
    elif x > 5:
        return 2
    else:
        return 1


def nested_guard(a, b, c):
    if a > 0:
        if b > 0:
            if c > 0:
                return 1
    return 0


def loop_filter(n):
    total = 0
    for i in range(n):
        if i % 2 == 0:
            total += i
    return total


def wide_params(a, b, c, d, e, f):
    return a + b + c + d + e + f


def bool_blend(a, b, c, d):
    if (a > 0 and b > 0) or (c > 0 and d > 0):
        return 1
    return 0
