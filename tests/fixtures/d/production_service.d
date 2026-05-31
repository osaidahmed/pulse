module production_service;

import std.conv;
import std.string;
import std.format;

class PaymentService {
    string gateway;
    string db;
    string cache;
    string logger;
    string notifier;
    string[string] config;
    int retryLimit;

    this(string gateway, string db, string cache, string logger, string notifier, string apiKey, int retryLimit, string p8, string p9) {
        this.gateway = gateway;
        this.db = db;
        this.cache = cache;
        this.logger = logger;
        this.notifier = notifier;
        this.config["apiKey"] = apiKey;
        this.retryLimit = retryLimit;
    }

    string processPayment(double amount, string currency, string method, string userID) {
        if (amount <= 0) {
            return "error: invalid amount";
        }
        if (currency == "") {
            return "error: missing currency";
        }

        if (method == "credit_card") {
            if (amount > 10000) {
                return "error: credit card limit exceeded";
            }
            if (currency != "USD" && currency != "EUR") {
                return "error: unsupported currency for credit card";
            }
            return format("cc_%s_%d", userID, 0);
        } else if (method == "bank_transfer") {
            if (amount < 100) {
                return "error: minimum amount for bank transfer is 100";
            }
            return format("bt_%s_%d", userID, 0);
        } else if (method == "crypto") {
            if (currency != "BTC" && currency != "ETH") {
                return "error: unsupported crypto currency";
            }
            return format("cr_%s_%d", userID, 0);
        } else if (method == "wallet") {
            return format("wl_%s_%d", userID, 0);
        } else {
            return "error: unknown payment method";
        }
    }

    string getStatus(string txID) {
        if (txID.startsWith("cc_")) {
            return "credit_card_pending";
        }
        return "unknown";
    }

    string refund(string txID) {
        if (txID == "") {
            return "error: empty transaction ID";
        }
        return "ok";
    }

    string getConfig(string key) {
        if (key in this.config) {
            return this.config[key];
        }
        return "";
    }
}

int processAlphaOrder(int x, int y) {
    int result = x + y;
    if (result > 100) {
        result = result - 50;
    }
    if (result < 0) {
        result = 0;
    }
    return result * 2;
}

int processBetaOrder(int a, int b) {
    int result = a + b;
    if (result > 100) {
        result = result - 50;
    }
    if (result < 0) {
        result = 0;
    }
    return result * 2;
}
