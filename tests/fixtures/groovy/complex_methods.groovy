class OrderProcessor {
    int processOrder(int status, int type, int priority, int quantity, int discount, int region, int category, int tier) {
        if (status == 1) {
            if (type > 0) {
                return 1
            }
        }
        if (status == 2) {
            if (priority > 5) {
                return 2
            }
        }
        if (quantity > 100 || discount > 50) {
            return 3
        }
        if (region == 1) {
            return 4
        }
        if (category > 3) {
            return 5
        }
        if (tier > 2 && priority > 3) {
            return 6
        }
        return 0
    }
}
