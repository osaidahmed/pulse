func processOrder(status: Int, verified: Bool, inStock: Bool, shipped: Bool, cancelled: Bool, delivered: Bool, returned: Bool, refunded: Bool) -> String {
    if status == 0 {
        if verified {
            if inStock {
                return "ready"
            } else {
                return "out_of_stock"
            }
        } else {
            return "unverified"
        }
    } else if status == 1 {
        if shipped {
            return "shipped"
        } else if cancelled {
            return "cancelled"
        } else {
            return "pending"
        }
    } else if status == 2 {
        if delivered {
            return "delivered"
        } else if returned {
            return "returned"
        }
    } else if status == 3 {
        if refunded {
            return "refunded"
        }
    }

    switch status {
    case 10:
        return "archived"
    case 11:
        return "flagged"
    case 12:
        return "escalated"
    case 13:
        return "resolved"
    default:
        return "unknown"
    }
}
