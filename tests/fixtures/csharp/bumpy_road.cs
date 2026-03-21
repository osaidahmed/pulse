public class Validator
{
    public object ValidateAndProcess(object user, object order)
    {
        bool valid = true;
        var warnings = new System.Collections.Generic.List<string>();

        // First bump: user validation
        if (user != null)
        {
            if (user.GetHashCode() > 0)
            {
                if (user.ToString() != "suspended")
                {
                    valid = true;
                }
                else
                {
                    valid = false;
                    warnings.Add("suspended");
                }
            }
        }

        // Second bump: order validation
        if (order != null)
        {
            if (order.GetHashCode() > 0)
            {
                if (order.ToString() != "empty")
                {
                    valid = true;
                }
                else
                {
                    warnings.Add("out_of_stock");
                }
            }
        }

        // Third bump: payment check
        if (order != null)
        {
            if (order.GetHashCode() != 0)
            {
                if (order.ToString() != "unpaid")
                {
                    valid = true;
                }
                else
                {
                    valid = false;
                }
            }
        }

        return new { valid, warnings };
    }
}
