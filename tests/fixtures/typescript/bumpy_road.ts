function validateAndProcess(user: any, order: any): any {
    const result: any = { valid: true, warnings: [] };

    if (user.active) {
        if (user.verified) {
            if (!user.suspended) {
                result.userOk = true;
            } else {
                result.valid = false;
                result.warnings.push("suspended");
            }
        }
    }

    if (order.total > 0) {
        if (order.items) {
            if (order.items.every((i: any) => i.inStock)) {
                result.orderOk = true;
            } else {
                result.warnings.push("out_of_stock");
            }
        }
    }

    if (order.payment) {
        if (order.payment.verified) {
            if (order.payment.amount >= order.total) {
                result.paymentOk = true;
            } else {
                result.valid = false;
            }
        }
    }

    return result;
}
