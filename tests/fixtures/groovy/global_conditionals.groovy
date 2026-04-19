if (System.getenv("MODE") == "debug") {
    if (System.getenv("VERBOSE") == "true") {
        if (System.getenv("TRACE") == "on") {
            if (System.getenv("LEVEL") == "all") {
                println("full debug")
            }
        }
    }
}

class Helper {
    void run() {
        println("running")
    }
}
