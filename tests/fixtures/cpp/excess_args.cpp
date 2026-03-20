#include <string>

void create_user(std::string first, std::string last, std::string email,
                 std::string phone, std::string addr, std::string city,
                 std::string state, std::string zip) {
}

int simple_func(int a, int b) {
    return a + b;
}

class UserService {
public:
    UserService(std::string db, std::string cache, std::string logger,
                std::string mailer, std::string validator, std::string scheduler)
        : db_(db), cache_(cache), logger_(logger),
          mailer_(mailer), validator_(validator), scheduler_(scheduler) {}

    std::string get_user(const std::string& id) {
        return this->db_;
    }

private:
    std::string db_;
    std::string cache_;
    std::string logger_;
    std::string mailer_;
    std::string validator_;
    std::string scheduler_;
};
