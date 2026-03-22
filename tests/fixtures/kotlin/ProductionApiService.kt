data class AppConfig(
    val host: String,
    val port: Int,
    val databaseUrl: String,
    val redisUrl: String,
    val jwtSecret: String,
    val logLevel: String,
    val maxConnections: Int,
    val timeoutSecs: Int,
    val retryCount: Int,
    val cacheTtl: Int,
    val rateLimit: Int,
    val uploadDir: String,
    val corsOrigin: String
)

class RequestHandler {
    fun dispatch(action: String): String {
        return when (action) {
            "create" -> "creating"
            "delete" -> "deleting"
            "update" -> "updating"
            "reset" -> "resetting"
            "notify" -> "notifying"
            "export" -> "exporting"
            else -> "unknown"
        }
    }

    fun processEvent(data: IntArray): Int {
        val a = data[0]
        val b = data[1]
        val c = data[2]
        val d = data[3]
        val e = data[4]
        val f = data[5]
        val g = data[6]
        val h = data[7]
        if (a > 100) {
            return -1
        }
        if (b > 200) {
            return -2
        }
        val result = a + b + c + d
        val extra = e + f + g + h
        return result + extra
    }
}
