if os.getenv("DEBUG") then
    print("debug mode")
end

function setup(config)
    if config.verbose then
        print("verbose")
    end
    if config.debug then
        print("debug")
    end
    return config
end
