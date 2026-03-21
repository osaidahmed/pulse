public class Calculator
{
    private int result;

    public int Add(int a, int b)
    {
        this.result = a + b;
        return this.result;
    }

    public int GetResult()
    {
        return this.result;
    }
}
