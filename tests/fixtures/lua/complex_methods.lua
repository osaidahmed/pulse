function processOrder(status, priority, amount, discount, tax)
    local result = 0
    if status == 1 then
        if priority > 5 then
            result = amount * 2
        elseif priority > 3 then
            result = amount
        else
            result = amount / 2
        end
    elseif status == 2 then
        if discount > 0 then
            result = amount - discount
        end
    elseif status == 3 then
        if priority == 1 then
            result = amount + tax
        elseif priority == 2 then
            result = amount - tax
        elseif priority == 3 then
            result = amount
        else
            result = 0
        end
    end
    if result < 0 then
        result = 0
    end
    return result
end

function simpleHelper(x)
    return x + 1
end
