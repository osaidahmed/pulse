export function flatCalls(): number {
    const a = 1;
    const b = 2;
    const c = a + b;
    return c;
}

export function pickBranch(x: number): number {
    if (x > 10) {
        return 3;
    } else if (x > 5) {
        return 2;
    } else {
        return 1;
    }
}

export function nestedGuard(a: number, b: number, c: number): number {
    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                return 1;
            }
        }
    }
    return 0;
}

export function loopFilter(n: number): number {
    let total = 0;
    for (let i = 0; i < n; i++) {
        if (i % 2 === 0) {
            total += i;
        }
    }
    return total;
}

export function wideParams(a: number, b: number, c: number, d: number, e: number, f: number): number {
    return a + b + c + d + e + f;
}

export function boolBlend(a: number, b: number, c: number, d: number): number {
    if ((a > 0 && b > 0) || (c > 0 && d > 0)) {
        return 1;
    }
    return 0;
}
