function validate(data, config, rules)
    local result = 0
    if data.len > 0 then
        if data.valid then
            if data.score > 10 then
                result = result + 1
            end
        end
    end
    local gap1 = 0
    local gap2 = 0
    if config.enabled then
        if config.strict then
            if config.level > 3 then
                result = result + 2
            end
        end
    end
    local gap3 = 0
    local gap4 = 0
    if rules.active then
        if rules.required then
            if rules.priority > 5 then
                result = result + 3
            end
        end
    end
    return result
end
