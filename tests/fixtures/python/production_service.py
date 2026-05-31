"""A realistic production service with multiple smell types for integration testing."""

import json
import logging
from datetime import datetime, timedelta
from typing import Optional

logger = logging.getLogger(__name__)


class UserService:
    """Manages user operations — intentionally has low cohesion for testing."""

    def __init__(self, db, cache, mailer, event_bus, rate_limiter, audit_logger, p7, p8, p9):
        self.db = db
        self.cache = cache
        self.mailer = mailer
        self.event_bus = event_bus
        self.rate_limiter = rate_limiter
        self.audit_logger = audit_logger

    def get_user(self, user_id):
        cached = self.cache.get(f"user:{user_id}")
        if cached:
            return cached
        user = self.db.users.get(user_id)
        if user:
            self.cache.set(f"user:{user_id}", user)
        return user

    def send_welcome_email(self, user):
        self.mailer.send(
            to=user.email,
            subject="Welcome",
            body=f"Hello {user.name}",
        )

    def publish_event(self, event_type, payload):
        self.event_bus.publish(event_type, payload)

    def check_rate_limit(self, user_id, action):
        return self.rate_limiter.check(user_id, action)

    def log_audit(self, user_id, action, details):
        self.audit_logger.log(user_id, action, details)


class OrderProcessor:
    """Complex order processing with multiple code smells."""

    def __init__(self, db, payment_gateway, inventory, shipping, notification):
        self.db = db
        self.payment_gateway = payment_gateway
        self.inventory = inventory
        self.shipping = shipping
        self.notification = notification

    def process_order(
        self,
        order_id: str,
        customer_id: str,
        items: list,
        shipping_address: str,
        billing_address: str,
        payment_method: str,
        coupon_code: Optional[str],
        gift_wrap: bool,
        delivery_notes: str,
        priority: str,
    ):
        order = self.db.orders.get(order_id)
        if order is None:
            return {"error": "order_not_found"}

        if order.status == "pending":
            if order.payment_verified:
                if self.inventory.check_all(items):
                    order.status = "processing"
                    for item in items:
                        if item.quantity > self.inventory.available(item.sku):
                            order.status = "partial_backorder"
                            break
                else:
                    order.status = "backordered"
            else:
                payment_result = self.payment_gateway.charge(
                    customer_id, order.total, payment_method
                )
                if payment_result.success:
                    order.payment_verified = True
                    order.status = "processing"
                elif payment_result.retriable:
                    order.status = "payment_pending"
                else:
                    order.status = "payment_failed"
        elif order.status == "processing":
            if self.shipping.ready(order_id):
                tracking = self.shipping.ship(order_id, shipping_address)
                order.tracking_number = tracking
                order.status = "shipped"
                self.notification.send(customer_id, f"Order {order_id} shipped")
            elif order.cancelled:
                self.inventory.release_all(items)
                order.status = "cancelled"
        elif order.status == "shipped":
            if order.delivered:
                order.status = "delivered"
                order.delivered_at = datetime.now()

        self.db.orders.save(order)
        return {"status": order.status}

    def generate_invoice(self, order_id):
        order = self.db.orders.get(order_id)
        items = self.db.order_items.get(order_id)
        customer = self.db.customers.get(order.customer_id)
        return {
            "invoice_number": f"INV-{order_id}",
            "date": datetime.now().isoformat(),
            "customer": customer.name,
            "items": [{"name": i.name, "qty": i.quantity, "price": i.price} for i in items],
            "total": order.total,
        }

    def generate_receipt(self, order_id):
        order = self.db.orders.get(order_id)
        items = self.db.order_items.get(order_id)
        customer = self.db.customers.get(order.customer_id)
        return {
            "receipt_number": f"RCP-{order_id}",
            "date": datetime.now().isoformat(),
            "customer": customer.name,
            "items": [{"name": i.name, "qty": i.quantity, "price": i.price} for i in items],
            "total": order.total,
        }


def format_user_report(users):
    report = []
    for user in users:
        entry = {}
        entry["id"] = user.id
        entry["name"] = user.name
        entry["email"] = user.email
        entry["status"] = "active" if user.active else "inactive"
        entry["joined"] = user.created_at.strftime("%Y-%m-%d")
        entry["display"] = f"{user.name} ({user.email})"
        report.append(entry)
    return report


def format_admin_report(admins):
    report = []
    for admin in admins:
        entry = {}
        entry["id"] = admin.id
        entry["name"] = admin.name
        entry["email"] = admin.email
        entry["status"] = "active" if admin.active else "inactive"
        entry["joined"] = admin.created_at.strftime("%Y-%m-%d")
        entry["display"] = f"{admin.name} ({admin.email})"
        report.append(entry)
    return report


def validate_batch(record):
    issues = []
    if record.user:
        if record.user.active:
            if record.user.banned:
                issues.append("banned_user")
    if record.payment:
        if record.payment.amount > 0:
            if not record.payment.authorized:
                issues.append("unauthorized")
    return issues
