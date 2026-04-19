class GodObject {
    int alpha
    int beta
    int gamma

    int workAlpha() { return this.alpha + 1 }
    int readAlpha() { return this.alpha * 2 }

    int workBeta() { return this.beta + 1 }
    int readBeta() { return this.beta * 2 }

    int workGamma() { return this.gamma + 1 }
    int readGamma() { return this.gamma * 2 }
}
