class ApiService {
    void handleApiRequest(int method, int path, int auth, int version, int format, int encoding, int cache) {
        if (method == 1) {
            if (path > 0) {
                if (auth == 1) {
                    if (version > 2) {
                        if (format == 1) {
                            println("json")
                        }
                    }
                }
            }
        }
        if (method == 2) {
            if (auth == 0) {
                return
            }
        }
        if (version > 3 || format > 2) {
            if (encoding == 1) {
                println("gzip")
            }
        }
        if (cache > 0 && method == 1) {
            println("cached")
        }
        if (method == 3) {
            if (path > 10) {
                if (auth == 1) {
                    println("delete")
                }
            }
        }
        if (method == 4) {
            println("patch")
        }
    }

    void processEvent(int type, int source) {
        int a = type
        int b = source
        int c = a + b
        int d = c * 2
        int e = d - 1
        int f = e + 3
        int g = f * a
        int h = g - b
        int i = h + 1
        int j = i * 2
        int k = j - 3
        int l = k + 4
        int m = l * 5
        int n = m - 6
        int o = n + 7
        int p = o * 8
        int q = p - 9
        int r = q + 10
        int s = r * 11
        int t = s - 12
        println(t)
    }

    void applyMiddleware(int data) {
        try {
            int x = data + 1
        } catch (Exception ex) {
        }
    }

    void parseQuery(int input) {
        if (input > 0) {
            if (input > 10) {
                if (input > 100) {
                    if (input > 1000) {
                        println("very large")
                    }
                }
            }
        }
    }

    int transformAlpha(int[] items) {
        int r = 0
        for (int v : items) { r += v }
        r = r * 2
        return r
    }

    int transformBeta(int[] items) {
        int r = 0
        for (int v : items) { r += v }
        r = r * 2
        return r
    }
}
