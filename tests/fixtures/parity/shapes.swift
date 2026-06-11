func flatCalls() -> Int {
    let a = 1
    let b = 2
    let c = a + b
    return c
}

func pickBranch(x: Int) -> Int {
    if x > 10 {
        return 3
    } else if x > 5 {
        return 2
    } else {
        return 1
    }
}

func nestedGuard(a: Int, b: Int, c: Int) -> Int {
    if a > 0 {
        if b > 0 {
            if c > 0 {
                return 1
            }
        }
    }
    return 0
}

func loopFilter(n: Int) -> Int {
    var total = 0
    for i in 0..<n {
        if i % 2 == 0 {
            total += i
        }
    }
    return total
}

func wideParams(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int) -> Int {
    return a + b + c + d + e + f
}

func boolBlend(a: Int, b: Int, c: Int, d: Int) -> Int {
    if (a > 0 && b > 0) || (c > 0 && d > 0) {
        return 1
    }
    return 0
}
