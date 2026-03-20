"""Functions heavy on primitive types without domain abstractions (future)."""


def create_invoice(
    customer_name: str,
    customer_email: str,
    customer_phone: str,
    customer_address: str,
    item_name: str,
    item_price: float,
    item_quantity: int,
    tax_rate: float,
    discount_code: str,
    shipping_method: str,
    notes: str,
):
    subtotal = item_price * item_quantity
    tax = subtotal * tax_rate
    total = subtotal + tax
    return {
        "customer": customer_name,
        "email": customer_email,
        "total": total,
    }


def send_notification(
    recipient_name: str,
    recipient_email: str,
    subject: str,
    body: str,
    priority: str,
    template: str,
    reply_to: str,
):
    pass
