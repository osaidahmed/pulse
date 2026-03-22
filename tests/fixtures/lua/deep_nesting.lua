function deeplyNested(items, threshold)
    local count = 0
    for _, item in ipairs(items) do
        if item > threshold then
            if item < 200 then
                if item ~= 128 then
                    if count < 1000 then
                        count = count + 1
                    end
                end
            end
        end
    end
    return count
end

function moderatelyNested(x, y)
    if x > 0 then
        if y > 0 then
            return x + y
        end
    end
    return 0
end
