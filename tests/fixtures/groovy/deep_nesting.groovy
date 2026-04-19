class Validator {
    void deeplyNested(int a, int b, int c, int d) {
        if (a > 0) {
            if (b > 0) {
                if (c > 0) {
                    if (d > 0) {
                        println("deep")
                    }
                }
            }
        }
    }

    void moderatelyNested(int a, int b) {
        if (a > 0) {
            if (b > 0) {
                println("moderate")
            }
        }
    }
}
