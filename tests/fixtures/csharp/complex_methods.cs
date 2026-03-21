public class OrderProcessor
{
    public string ProcessOrder(int status, bool verified, bool inStock,
                               bool shipped, bool cancelled, bool delivered)
    {
        if (status == 0)
        {
            if (verified)
            {
                if (inStock)
                {
                    status = 1;
                }
                else
                {
                    status = 2;
                }
            }
            else
            {
                status = 3;
            }
        }
        else if (status == 1)
        {
            if (shipped)
            {
                status = 4;
            }
            else if (cancelled)
            {
                status = 5;
            }
        }
        else if (status == 2)
        {
            if (verified && inStock)
            {
                status = 1;
            }
            else if (cancelled)
            {
                status = 5;
            }
        }
        else if (status == 4)
        {
            if (delivered)
            {
                status = 6;
            }
        }
        return status.ToString();
    }
}
