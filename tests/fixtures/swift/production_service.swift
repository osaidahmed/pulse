class OrderService {
    var orders: [String: Int] = [:]
    var customers: [String: String] = [:]
    var inventory: [String: Int] = [:]

    func processOrder(orderId: String, customerId: String, items: [String], quantities: [Int], priority: Int, discount: Double) -> String {
        guard !orderId.isEmpty else { return "invalid_order" }
        guard !customerId.isEmpty else { return "invalid_customer" }

        if items.count != quantities.count {
            return "mismatch"
        }

        var total = 0
        for i in 0..<items.count {
            if let stock = inventory[items[i]] {
                if stock >= quantities[i] {
                    total += quantities[i]
                } else {
                    return "insufficient_stock"
                }
            } else {
                return "item_not_found"
            }
        }

        if priority > 5 {
            if discount > 0.5 {
                return "excessive_discount"
            } else if discount > 0.2 {
                total = Int(Double(total) * (1.0 - discount))
            }
        } else if priority > 3 {
            if discount > 0.3 {
                return "discount_too_high"
            }
        }

        switch priority {
        case 1:
            orders[orderId] = total
        case 2:
            orders[orderId] = total * 2
        case 3:
            orders[orderId] = total * 3
        default:
            orders[orderId] = total
        }

        return "success"
    }

    func validateCustomer(id: String, name: String, email: String, phone: String, address: String, zipCode: String) -> Bool {
        if id.isEmpty || name.isEmpty {
            return false
        }
        if email.contains("@") && email.contains(".") {
            if phone.count >= 10 {
                if !address.isEmpty && !zipCode.isEmpty {
                    return true
                }
            }
        }
        return false
    }

    func generateReport(startDate: String, endDate: String) -> String {
        var report = "Report from \(startDate) to \(endDate)\n"
        for (orderId, total) in orders {
            if total > 100 {
                report += "HIGH: \(orderId) = \(total)\n"
            } else if total > 50 {
                report += "MED: \(orderId) = \(total)\n"
            } else {
                report += "LOW: \(orderId) = \(total)\n"
            }
        }
        return report
    }
}
