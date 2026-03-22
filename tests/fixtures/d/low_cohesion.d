module low_cohesion;

class LowCohesion {
    int x;
    int y;
    string name;
    string email;
    double score;
    double weight;

    void updatePosition() {
        this.x = this.x + 1;
        this.y = this.y + 1;
    }

    void resetPosition() {
        this.x = 0;
        this.y = 0;
    }

    string formatContact() {
        return this.name ~ " <" ~ this.email ~ ">";
    }

    string getEmail() {
        return this.email;
    }

    double computeWeighted() {
        return this.score * this.weight;
    }

    double normalizeScore() {
        return this.score / 100.0;
    }
}
