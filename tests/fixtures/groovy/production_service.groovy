class OrderService {
    OrderService(Object db, Object cache, Object logger, Object validator, Object notifier, Object formatter, Object auditor) {
        println("init")
    }

    int processOrder(int status, int type, int priority, int qty, int discount, int region, int category) {
        if (status == 1) {
            if (type > 0) {
                if (priority > 5) {
                    if (qty > 100) {
                        return 1
                    }
                }
            }
        }
        if (status == 2) {
            if (discount > 50) {
                return 2
            }
        }
        if (region == 1 || region == 2) {
            return 3
        }
        if (category > 3 && priority > 2) {
            return 4
        }
        if (qty > 500) {
            return 5
        }
        return 0
    }

    int processDataPipeline(int[] data, int mode, int flags, int threshold, int limit, int offset, int batch) {
        if (mode == 1) {
            if (flags > 0) {
                return 1
            }
        }
        if (mode == 2) {
            if (threshold > 100) {
                return 2
            }
        }
        if (limit > 0 && offset > 0) {
            return 3
        }
        if (batch > 10) {
            if (data.length > 100) {
                return 4
            }
        }
        if (flags == 3 || flags == 7) {
            return 5
        }
        if (data.length > 1000) {
            return 6
        }
        return 0
    }

    int validateSchema(int x) {
        return x + 1
    }

    int computeAlpha(int x, int y) {
        int result = x + y
        if (result > 100) { result = result - 50 }
        if (result < 0) { result = 0 }
        return result * 2
    }

    int computeBeta(int a, int b) {
        int result = a + b
        if (result > 100) { result = result - 50 }
        if (result < 0) { result = 0 }
        return result * 2
    }

    int validateBatch(int user, int active, int banned, int payment, int amount) {
        int issues = 0
        if (user == 1) {
            if (active == 1) {
                if (banned == 1) {
                    issues = issues + 1
                }
            }
        }
        if (payment == 1) {
            if (amount > 0) {
                if (amount < 10) {
                    issues = issues + 1
                }
            }
        }
        return issues
    }
}
