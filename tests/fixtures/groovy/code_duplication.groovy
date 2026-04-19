class DataProcessor {
    int processAlpha(int x, int y) {
        int result = x + y
        if (result > 100) { result = result - 50 }
        if (result < 0) { result = 0 }
        return result * 2
    }

    int processBeta(int a, int b) {
        int result = a + b
        if (result > 100) { result = result - 50 }
        if (result < 0) { result = 0 }
        return result * 2
    }
}
