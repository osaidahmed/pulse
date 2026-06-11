int flat_calls(void) {
    int a = 1;
    int b = 2;
    int c = a + b;
    return c;
}

int pick_branch(int x) {
    if (x > 10) {
        return 3;
    } else if (x > 5) {
        return 2;
    } else {
        return 1;
    }
}

int nested_guard(int a, int b, int c) {
    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                return 1;
            }
        }
    }
    return 0;
}

int loop_filter(int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        if (i % 2 == 0) {
            total += i;
        }
    }
    return total;
}

int wide_params(int a, int b, int c, int d, int e, int f) {
    return a + b + c + d + e + f;
}

int bool_blend(int a, int b, int c, int d) {
    if ((a > 0 && b > 0) || (c > 0 && d > 0)) {
        return 1;
    }
    return 0;
}
