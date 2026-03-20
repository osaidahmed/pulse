fn process_order(order: &mut Order) -> Option<&Order> {
    if order.status == Status::Pending {
        if order.payment_verified {
            if order.items_in_stock {
                order.status = Status::Processing;
            } else {
                order.status = Status::Backordered;
            }
        } else {
            order.status = Status::PaymentFailed;
        }
    } else if order.status == Status::Processing {
        if order.shipped {
            order.status = Status::Shipped;
        } else if order.cancelled {
            order.status = Status::Cancelled;
        }
    } else if order.status == Status::Shipped {
        if order.delivered {
            order.status = Status::Delivered;
        }
    }
    Some(order)
}

struct Order {
    status: Status,
    payment_verified: bool,
    items_in_stock: bool,
    shipped: bool,
    cancelled: bool,
    delivered: bool,
}

enum Status {
    Pending,
    Processing,
    Backordered,
    PaymentFailed,
    Shipped,
    Cancelled,
    Delivered,
}
