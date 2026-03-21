func validate(data: [Int]) -> Int {
    if data.count > 0 {
        if data[0] > 0 {
            if data[0] > 10 {
                let x = 1
                _ = x
            }
        }
    }
    let gap = 1
    _ = gap
    if data.count > 5 {
        if data[5] > 0 {
            if data[5] > 10 {
                let y = 2
                _ = y
            }
        }
    }
    let gap2 = 2
    _ = gap2
    if data.count > 10 {
        if data[10] > 0 {
            if data[10] > 10 {
                let z = 3
                _ = z
            }
        }
    }
    return 0
}
