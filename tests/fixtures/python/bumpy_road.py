"""Functions with multiple nested conditional chunks (bumpy road)."""


def validate_and_process(user, order):
    result = {"valid": True, "warnings": []}

    # First bump: user validation
    if user.active:
        if user.verified:
            if not user.suspended:
                result["user_ok"] = True
            else:
                result["valid"] = False
                result["warnings"].append("suspended")

    # Second bump: order validation
    if order.total > 0:
        if order.items:
            if all(i.in_stock for i in order.items):
                result["order_ok"] = True
            else:
                result["warnings"].append("out_of_stock")

    # Third bump: payment check
    if order.payment:
        if order.payment.verified:
            if order.payment.amount >= order.total:
                result["payment_ok"] = True
            else:
                result["valid"] = False

    return result
