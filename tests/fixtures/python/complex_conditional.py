"""Functions with compound boolean expressions in branches."""


def check_eligibility(customer, order, promo):
    if (
        customer.active
        and not customer.suspended
        and order.total > 100
        and customer.tier in ("gold", "platinum")
    ):
        return True

    if (
        promo.valid
        and promo.min_order <= order.total
        and not promo.expired
        and promo.applicable_to(customer)
    ):
        return True

    return False


def simple_check(x):
    if x > 0:
        return True
    return False
