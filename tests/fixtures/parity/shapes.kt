fun flatCalls(): Int {
    val a = 1
    val b = 2
    val c = a + b
    return c
}

fun pickBranch(x: Int): Int {
    if (x > 10) {
        return 3
    } else if (x > 5) {
        return 2
    } else {
        return 1
    }
}

fun nestedGuard(a: Int, b: Int, c: Int): Int {
    if (a > 0) {
        if (b > 0) {
            if (c > 0) {
                return 1
            }
        }
    }
    return 0
}

fun loopFilter(n: Int): Int {
    var total = 0
    for (i in 0 until n) {
        if (i % 2 == 0) {
            total += i
        }
    }
    return total
}

fun wideParams(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int): Int {
    return a + b + c + d + e + f
}

fun boolBlend(a: Int, b: Int, c: Int, d: Int): Int {
    if ((a > 0 && b > 0) || (c > 0 && d > 0)) {
        return 1
    }
    return 0
}
