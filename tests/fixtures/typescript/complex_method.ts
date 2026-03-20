function processOrder(order: any): any {
    if (order === null) {
        return null;
    }
    if (order.status === "pending") {
        if (order.paymentVerified) {
            if (order.itemsInStock) {
                order.status = "processing";
            } else {
                order.status = "backordered";
            }
        } else {
            order.status = "payment_failed";
        }
    } else if (order.status === "processing") {
        if (order.shipped) {
            order.status = "shipped";
        } else if (order.cancelled) {
            order.status = "cancelled";
        }
    } else if (order.status === "shipped") {
        if (order.delivered) {
            order.status = "delivered";
        }
    }
    return order;
}
