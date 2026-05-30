int classify(int x) {
    static if (is(typeof(x) == int)) {
        if (x > 0) {
            return 1;
        }
        return -1;
    } else {
        return 0;
    }
}

version (Posix) {
    int platform() {
        return 1;
    }
} else {
    int platform() {
        return 0;
    }
}
