class DeepNesting {
    fun deeplyNested(data: List<List<List<Int>>>): Boolean {
        for (group in data) {
            if (group.isNotEmpty()) {
                for (items in group) {
                    if (items.isNotEmpty()) {
                        for (v in items) {
                            if (v > 0) {
                                println(v)
                            }
                        }
                    }
                }
            }
        }
        return true
    }

    fun moderatelyNested(data: List<Int>): Int {
        var sum = 0
        for (item in data) {
            if (item > 0) {
                sum += item
            }
        }
        return sum
    }
}
