pub fn flatCalls() i32 {
    const a: i32 = 1;
    const b: i32 = 2;
    const c = a + b;
    return c;
}

pub fn pickBranch(x: i32) i32 {
    if (x > 10) {
        return 3;
    } else if (x > 5) {
        return 2;
    } else {
        return 1;
    }
}

pub fn nestedGuard(a: i32, b: i32, c: i32) i32 {
    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                return 1;
            }
        }
    }
    return 0;
}

pub fn loopFilter(n: i32) i32 {
    var total: i32 = 0;
    var i: i32 = 0;
    while (i < n) {
        if (@mod(i, 2) == 0) {
            total += i;
        }
        i += 1;
    }
    return total;
}

pub fn wideParams(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) i32 {
    return a + b + c + d + e + f;
}

pub fn boolBlend(a: i32, b: i32, c: i32, d: i32) i32 {
    if ((a > 0 and b > 0) or (c > 0 and d > 0)) {
        return 1;
    }
    return 0;
}
