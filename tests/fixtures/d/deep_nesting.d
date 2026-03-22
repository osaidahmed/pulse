module deep_nesting;

bool deeplyNested(int[][] data) {
    foreach (group; data) {
        if (group.length > 0) {
            foreach (item; group) {
                if (item > 0) {
                    if (item % 2 == 0) {
                        return true;
                    }
                }
            }
        }
    }
    return false;
}

bool moderatelyNested(int[] data) {
    foreach (item; data) {
        if (item > 0) {
            return true;
        }
    }
    return false;
}
