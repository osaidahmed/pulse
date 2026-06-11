function flatCalls() {
    const a = 1;
    const b = 2;
    const c = a + b;
    return c;
}

function pickBranch(x) {
    if (x > 10) {
        return 3;
    } else if (x > 5) {
        return 2;
    } else {
        return 1;
    }
}

function nestedGuard(a, b, c) {
    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                return 1;
            }
        }
    }
    return 0;
}

function loopFilter(n) {
    let total = 0;
    for (let i = 0; i < n; i++) {
        if (i % 2 === 0) {
            total += i;
        }
    }
    return total;
}

function wideParams(a, b, c, d, e, f) {
    return a + b + c + d + e + f;
}

function boolBlend(a, b, c, d) {
    if ((a > 0 && b > 0) || (c > 0 && d > 0)) {
        return 1;
    }
    return 0;
}
