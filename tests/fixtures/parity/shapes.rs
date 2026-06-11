pub fn flat_calls() -> i32 {
    let a = 1;
    let b = 2;
    let c = a + b;
    c
}

pub fn pick_branch(x: i32) -> i32 {
    if x > 10 {
        return 3;
    } else if x > 5 {
        return 2;
    } else {
        return 1;
    }
}

pub fn nested_guard(a: i32, b: i32, c: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            if c > 0 {
                return 1;
            }
        }
    }
    0
}

pub fn loop_filter(n: i32) -> i32 {
    let mut total = 0;
    for i in 0..n {
        if i % 2 == 0 {
            total += i;
        }
    }
    total
}

pub fn wide_params(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
    a + b + c + d + e + f
}

pub fn bool_blend(a: i32, b: i32, c: i32, d: i32) -> i32 {
    if (a > 0 && b > 0) || (c > 0 && d > 0) {
        return 1;
    }
    0
}
