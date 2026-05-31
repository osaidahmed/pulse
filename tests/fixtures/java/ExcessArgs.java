public class ExcessArgs {
    public static String createUser(String first, String last, String email,
                                     String phone, String addr, String city,
                                     String state, String zip) {
        return first + " " + last;
    }

    public static int simpleFunc(int a, int b) {
        return a + b;
    }
}

class UserService {
    private String db;
    private String cache;
    private String logger;
    private String mailer;
    private String validator;
    private String scheduler;

    public UserService(String db, String cache, String logger,
                       String mailer, String validator, String scheduler,
                       String extra7, String extra8, String extra9) {
        this.db = db;
        this.cache = cache;
        this.logger = logger;
        this.mailer = mailer;
        this.validator = validator;
        this.scheduler = scheduler;
    }

    public String getUser(String userId) {
        return this.db;
    }
}
