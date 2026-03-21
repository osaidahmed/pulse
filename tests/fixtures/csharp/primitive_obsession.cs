public class InvoiceService
{
    public object CreateInvoice(string customerName, string customerEmail,
                                string customerPhone, string customerAddress,
                                string itemName, double itemPrice,
                                int itemQuantity, double taxRate,
                                string discountCode, string shippingMethod,
                                string notes)
    {
        double subtotal = itemPrice * itemQuantity;
        double tax = subtotal * taxRate;
        double total = subtotal + tax;
        return new { customer = customerName, email = customerEmail, total };
    }

    public void SendNotification(string recipientName, string recipientEmail,
                                 string subject, string body,
                                 string priority, string template,
                                 string replyTo)
    {
    }
}
