-- A realistic Lua production service with multiple smell types for integration testing.

local OrderService = {}

function OrderService:new(db, cache, mailer, event_bus, rate_limiter, audit_logger, p7, p8, p9)
    self.db = db
    self.cache = cache
    self.mailer = mailer
    self.event_bus = event_bus
    self.rate_limiter = rate_limiter
    self.audit_logger = audit_logger
end

function OrderService:getOrder(order_id)
    local cached = self.cache:get("order:" .. order_id)
    if cached then
        return cached
    end
    local order = self.db:query("orders", order_id)
    if order then
        self.cache:set("order:" .. order_id, order)
    end
    return order
end

function OrderService:sendConfirmation(order)
    self.mailer:send(order.email, "Order Confirmed", "Your order is confirmed")
end

function OrderService:publishEvent(event_type, payload)
    self.event_bus:publish(event_type, payload)
end

function OrderService:checkRateLimit(user_id, action)
    return self.rate_limiter:check(user_id, action)
end

function OrderService:logAudit(user_id, action, details)
    self.audit_logger:log(user_id, action, details)
end

function OrderService:processOrder(
    order_id, customer_id, items, shipping_addr,
    billing_addr, payment_method, coupon, gift_wrap,
    delivery_notes, priority
)
    local order = self.db:query("orders", order_id)
    if order == nil then
        return { error = "order_not_found" }
    end

    if order.status == "pending" then
        if order.payment_verified then
            if self:checkInventory(items) then
                order.status = "processing"
                for _, item in ipairs(items) do
                    if item.quantity > self:available(item.sku) then
                        order.status = "partial_backorder"
                        break
                    end
                end
            else
                order.status = "backordered"
            end
        else
            local result = self:chargePayment(customer_id, order.total, payment_method)
            if result.success then
                order.payment_verified = true
                order.status = "processing"
            elseif result.retriable then
                order.status = "payment_pending"
            else
                order.status = "payment_failed"
            end
        end
    elseif order.status == "processing" then
        if self:shippingReady(order_id) then
            local tracking = self:shipOrder(order_id, shipping_addr)
            order.tracking_number = tracking
            order.status = "shipped"
            self:sendNotification(customer_id, "Order " .. order_id .. " shipped")
        elseif order.cancelled then
            self:releaseInventory(items)
            order.status = "cancelled"
        end
    elseif order.status == "shipped" then
        if order.delivered then
            order.status = "delivered"
            order.delivered_at = os.time()
        end
    end

    self.db:save("orders", order)
    return { status = order.status }
end

function OrderService:generateInvoice(order_id)
    local order = self.db:query("orders", order_id)
    local items = self.db:query("order_items", order_id)
    local customer = self.db:query("customers", order.customer_id)
    local line_items = {}
    for _, item in ipairs(items) do
        table.insert(line_items, {
            name = item.name,
            qty = item.quantity,
            price = item.price,
        })
    end
    return {
        invoice_number = "INV-" .. order_id,
        date = os.date("%Y-%m-%d"),
        customer = customer.name,
        items = line_items,
        total = order.total,
    }
end

function OrderService:generateReceipt(order_id)
    local order = self.db:query("orders", order_id)
    local items = self.db:query("order_items", order_id)
    local customer = self.db:query("customers", order.customer_id)
    local line_items = {}
    for _, item in ipairs(items) do
        table.insert(line_items, {
            name = item.name,
            qty = item.quantity,
            price = item.price,
        })
    end
    return {
        receipt_number = "RCP-" .. order_id,
        date = os.date("%Y-%m-%d"),
        customer = customer.name,
        items = line_items,
        total = order.total,
    }
end

function OrderService:checkInventory(items)
    return #items > 0
end

function OrderService:available(sku)
    return 100
end

function OrderService:chargePayment(customer_id, total, method)
    return { success = true }
end

function OrderService:shippingReady(order_id)
    return true
end

function OrderService:shipOrder(order_id, addr)
    return "TRK-" .. order_id
end

function OrderService:sendNotification(customer_id, msg)
    self.mailer:send(customer_id, "Notification", msg)
end

function OrderService:releaseInventory(items)
    for _, item in ipairs(items) do
        self.db:update("inventory", item.sku, item.quantity)
    end
end

local function formatUserReport(users)
    local report = {}
    for _, user in ipairs(users) do
        local entry = {}
        entry.id = user.id
        entry.name = user.name
        entry.email = user.email
        entry.status = user.active and "active" or "inactive"
        entry.joined = user.created_at
        entry.display = user.name .. " (" .. user.email .. ")"
        table.insert(report, entry)
    end
    return report
end

local function formatAdminReport(admins)
    local report = {}
    for _, admin in ipairs(admins) do
        local entry = {}
        entry.id = admin.id
        entry.name = admin.name
        entry.email = admin.email
        entry.status = admin.active and "active" or "inactive"
        entry.joined = admin.created_at
        entry.display = admin.name .. " (" .. admin.email .. ")"
        table.insert(report, entry)
    end
    return report
end


local function validateBatch(record)
    local issues = {}
    if record.user then
        if record.user.active then
            if record.user.banned then
                table.insert(issues, "banned_user")
            end
        end
    end
    if record.payment then
        if record.payment.amount > 0 then
            if not record.payment.authorized then
                table.insert(issues, "unauthorized")
            end
        end
    end
    return issues
end
