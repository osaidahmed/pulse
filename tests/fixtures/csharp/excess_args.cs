public class UserManager
{
    public object CreateUser(string firstName, string lastName, string email,
                             string phone, string address, string city,
                             string state, string zipcode)
    {
        return new { firstName, lastName, email, phone, address, city, state, zipcode };
    }

    public int SimpleFunc(int a, int b)
    {
        return a + b;
    }
}

public class UserService
{
    private object db;
    private object cache;
    private object logger;
    private object mailer;
    private object validator;
    private object scheduler;

    public UserService(object db, object cache, object logger,
                       object mailer, object validator, object scheduler,
                       object extra7, object extra8, object extra9)
    {
        this.db = db;
        this.cache = cache;
        this.logger = logger;
        this.mailer = mailer;
        this.validator = validator;
        this.scheduler = scheduler;
    }

    public object GetUser(string userId)
    {
        return null;
    }
}
