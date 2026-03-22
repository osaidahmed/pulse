module test_smells;

int add(int a, int b) {
    return a + b;
}

unittest {
    assert(add(1, 2) == 3);
    assert(add(0, 0) == 0);
    assert(add(-1, 1) == 0);
}

unittest {
    assert(add(100, 200) == 300);
}
