function test_user_creation()
    local u1 = createUser("alice")
    assert(u1.name == "alice")
    assert(u1.active == true)
    assert(u1.score == 0)
    assert(u1.level == 1)
    assert(u1.role == "user")
    assert(u1.email ~= nil)
    assert(u1.age >= 0)
    assert(u1.department == "default")
    assert(u1.region == "us")
    assert(u1.id ~= nil)
    assert(u1.created ~= nil)
    assert(u1.updated ~= nil)
end

function test_user_update()
    local u = createUser("bob")
    u.name = "robert"
    assert(u.name == "robert")
    assert(u.active == true)
end
