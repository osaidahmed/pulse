"""Function with high cyclomatic complexity (cc >= 9)."""


def process_order(order):
    if order is None:
        return None
    if order.status == "pending":
        if order.payment_verified:
            if order.items_in_stock:
                order.status = "processing"
            else:
                order.status = "backordered"
        else:
            order.status = "payment_failed"
    elif order.status == "processing":
        if order.shipped:
            order.status = "shipped"
        elif order.cancelled:
            order.status = "cancelled"
    elif order.status == "shipped":
        if order.delivered:
            order.status = "delivered"

    return order
