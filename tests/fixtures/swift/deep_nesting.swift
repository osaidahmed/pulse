func deeplyNested(x: Int, items: [Int]) {
    if x > 0 {
        for i in items {
            if i > 10 {
                if i > 100 {
                    if i > 1000 {
                        print("deep")
                    }
                }
            }
        }
    }
}

func moderatelyNested(x: Int) -> Int {
    if x > 0 {
        if x > 10 {
            return x
        }
    }
    return 0
}
