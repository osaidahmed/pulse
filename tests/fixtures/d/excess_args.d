module excess_args;

string createRecord(string name, string email, int age, string address, string phone, string role) {
    return name ~ email ~ address ~ phone ~ role;
}

string simple(int x) {
    return "ok";
}
