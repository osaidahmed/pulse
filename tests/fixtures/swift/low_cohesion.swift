class Kitchen {
    var oven: Int = 0
    var sink: Int = 0
    var fridge: Int = 0
    var dishwasher: Int = 0
    var microwave: Int = 0
    var blender: Int = 0

    func useOven() -> Int {
        return self.oven + 1
    }

    func useSink() -> Int {
        return self.sink + 1
    }

    func useFridge() -> Int {
        return self.fridge + 1
    }

    func useDishwasher() -> Int {
        return self.dishwasher + 1
    }

    func useMicrowave() -> Int {
        return self.microwave + 1
    }

    func useBlender() -> Int {
        return self.blender + 1
    }
}
