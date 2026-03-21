public class Processor
{
    public void DeeplyNested(object[] data)
    {
        foreach (var group in data)
        {
            if (group != null)
            {
                foreach (var item in new object[] { group })
                {
                    if (item != null)
                    {
                        foreach (var sub in new object[] { item })
                        {
                            if (sub != null)
                            {
                                Process(sub);
                            }
                        }
                    }
                }
            }
        }
    }

    public void ModeratelyNested(object[] data)
    {
        foreach (var item in data)
        {
            if (item != null)
            {
                foreach (var sub in new object[] { item })
                {
                    Process(sub);
                }
            }
        }
    }

    private void Process(object x) { }
}
