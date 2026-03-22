class WhenExpression {
    fun dispatch(action: String): String {
        return when (action) {
            "create" -> "creating"
            "delete" -> "deleting"
            "update" -> "updating"
            "reset" -> "resetting"
            "notify" -> "notifying"
            "export" -> "exporting"
            "import" -> "importing"
            "validate" -> "validating"
            "transform" -> "transforming"
            else -> "unknown"
        }
    }
}
