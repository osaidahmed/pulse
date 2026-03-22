fun String.isEmail(): Boolean {
    return this.contains("@")
}

fun topLevel(a: Int, b: Int): Int = a + b

data class Point(val x: Double, val y: Double) {
    fun distanceTo(other: Point): Double {
        val dx = x - other.x
        val dy = y - other.y
        return Math.sqrt(dx * dx + dy * dy)
    }
}

object Registry {
    fun register(name: String): Boolean {
        return name.isNotEmpty()
    }
}

class Container(val items: List<String>) {
    companion object {
        fun empty(): Container {
            return Container(emptyList())
        }

        fun of(vararg items: String): Container {
            return Container(items.toList())
        }
    }

    init {
        println("Created container with ${items.size} items")
    }

    fun size(): Int {
        return items.size
    }
}
