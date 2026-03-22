module clean;

int add(int a, int b) {
    return a + b;
}

string greet(string name) {
    return "Hello, " ~ name;
}

int max(int a, int b) {
    if (a > b) {
        return a;
    }
    return b;
}
