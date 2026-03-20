"""A clean file with no smells. Should produce zero findings."""


def add(a, b):
    return a + b


def greet(name):
    return f"Hello, {name}"


class Calculator:
    def __init__(self, value=0):
        self.value = value

    def add(self, n):
        self.value += n
        return self

    def subtract(self, n):
        self.value -= n
        return self

    def result(self):
        return self.value
