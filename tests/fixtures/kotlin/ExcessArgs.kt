fun createUser(
    first: String,
    last: String,
    email: String,
    phone: String,
    addr: String,
    city: String,
    state: String,
    zip: String
): String {
    return "$first $last"
}

fun simpleFunc(a: Int, b: Int): Int {
    return a + b
}

class UserService(
    private val db: String,
    private val cache: String,
    private val logger: String,
    private val mailer: String,
    private val validator: String,
    private val scheduler: String,
    private val extra7: String,
    private val extra8: String,
    private val extra9: String
) {
    fun getUser(userId: String): String {
        return db
    }
}
