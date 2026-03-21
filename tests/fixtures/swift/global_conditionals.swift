let debug = true
let verbose = false

if debug {
    if verbose {
        print("verbose debug mode")
    }
}

if debug {
    print("debug on")
}

if verbose {
    print("verbose on")
}

if debug {
    if verbose {
        print("both")
    }
}

func helper() -> Int {
    return 42
}
