class ComplexMethod {
    fun processOrder(
        status: Int,
        verified: Boolean,
        stock: Boolean,
        shipped: Boolean,
        cancelled: Boolean,
        delivered: Boolean
    ): String {
        var s = status
        if (s == 0) {
            if (verified) {
                if (stock) {
                    s = 1
                } else {
                    s = 2
                }
            } else {
                s = 3
            }
        } else if (s == 1) {
            if (shipped) {
                s = 4
            } else if (cancelled) {
                s = 5
            }
        } else if (s == 4) {
            if (delivered) {
                s = 6
            }
        }
        return s.toString()
    }
}
